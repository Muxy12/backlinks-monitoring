//! SSRF address classification — the pure heart of the crawler's SSRF guard
//! (docs/06 §2, docs/11 S1). **The highest-severity control in the product.**
//!
//! This module has no I/O. It answers one question: *given an already-resolved
//! IP address (or a host/scheme/port), is it safe to fetch?* DNS resolution,
//! address pinning and redirect re-checking live in `crates/crawler` and call
//! into here — but the classification logic, which is where a mistake becomes a
//! vulnerability, is pure and exhaustively testable, so it lives in core
//! (ADR-0001). The crawler and the public checker (§50) share this one
//! implementation.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Why an address or host was rejected. Carried back to the user so they can
/// see *why* a check could not run (docs/06 §2.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrfBlock {
    Loopback,
    Private,
    LinkLocal,
    /// Cloud metadata endpoints (AWS/GCP/Azure/Alibaba). The crown-jewel target.
    Metadata,
    Cgnat,
    Unspecified,
    Broadcast,
    Multicast,
    /// IANA-reserved / documentation / benchmarking ranges.
    Reserved,
    /// IPv6 unique-local (fc00::/7).
    UniqueLocalV6,
    /// A blocked host name (localhost, *.internal, bare single-label, …).
    BlockedHost,
    /// Scheme other than http/https, or credentials embedded in the URL.
    BadScheme,
    /// A port outside the allowlist.
    BadPort,
}

/// Ports we permit. 80/443 plus the scheme defaults (which are the same).
const ALLOWED_PORTS: &[u16] = &[80, 443];

/// Classify a **single already-resolved** IP address. `Ok(())` means the
/// address is a public, routable destination we may fetch. Every resolved
/// address for a host must pass — the caller checks all of them (docs/06 §2.2).
pub fn classify_ip(ip: IpAddr) -> Result<(), SsrfBlock> {
    match ip {
        IpAddr::V4(v4) => classify_v4(v4),
        IpAddr::V6(v6) => classify_v6(v6),
    }
}

fn classify_v4(ip: Ipv4Addr) -> Result<(), SsrfBlock> {
    let o = ip.octets();

    // The metadata address is the single most important block. Check it first
    // and explicitly so it can never be masked by a broader rule's label.
    if ip == Ipv4Addr::new(169, 254, 169, 254) || ip == Ipv4Addr::new(100, 100, 100, 200) {
        return Err(SsrfBlock::Metadata);
    }
    if ip.is_loopback() {
        return Err(SsrfBlock::Loopback); // 127.0.0.0/8
    }
    if ip.is_private() {
        return Err(SsrfBlock::Private); // 10/8, 172.16/12, 192.168/16
    }
    if ip.is_link_local() {
        return Err(SsrfBlock::LinkLocal); // 169.254.0.0/16
    }
    if ip.is_unspecified() || o[0] == 0 {
        return Err(SsrfBlock::Unspecified); // 0.0.0.0/8
    }
    if ip.is_broadcast() {
        return Err(SsrfBlock::Broadcast); // 255.255.255.255
    }
    if ip.is_multicast() {
        return Err(SsrfBlock::Multicast); // 224.0.0.0/4
    }
    // CGNAT 100.64.0.0/10
    if o[0] == 100 && (64..=127).contains(&o[1]) {
        return Err(SsrfBlock::Cgnat);
    }
    // Reserved / documentation / benchmarking.
    if o[0] >= 240                                             // 240.0.0.0/4 (incl. 255/8)
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)            // 192.0.0.0/24
        || (o[0] == 192 && o[1] == 0 && o[2] == 2)            // 192.0.2.0/24  TEST-NET-1
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)         // 198.51.100.0/24 TEST-NET-2
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)          // 203.0.113.0/24 TEST-NET-3
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19))
    // 198.18.0.0/15 benchmarking
    {
        return Err(SsrfBlock::Reserved);
    }
    Ok(())
}

fn classify_v6(ip: Ipv6Addr) -> Result<(), SsrfBlock> {
    // Unwrap disguised IPv4 and re-check against the v4 rules. `::ffff:127.0.0.1`
    // is loopback wearing a hat, and a naive v6 check would pass it (docs/06 §2.2).
    if let Some(v4) = ip.to_ipv4_mapped() {
        return classify_v4(v4);
    }
    // 6to4 (2002::/16) embeds a v4 address in bytes 2..6 — extract and re-check.
    let seg = ip.segments();
    if seg[0] == 0x2002 {
        let o = ip.octets();
        let embedded = Ipv4Addr::new(o[2], o[3], o[4], o[5]);
        return classify_v4(embedded).map_err(|_| SsrfBlock::Reserved);
    }

    if ip.is_loopback() {
        return Err(SsrfBlock::Loopback); // ::1
    }
    if ip.is_unspecified() {
        return Err(SsrfBlock::Unspecified); // ::
    }
    if ip.is_multicast() {
        return Err(SsrfBlock::Multicast); // ff00::/8
    }
    // Link-local unicast fe80::/10.
    if seg[0] & 0xffc0 == 0xfe80 {
        return Err(SsrfBlock::LinkLocal);
    }
    // Unique local fc00::/7.
    if seg[0] & 0xfe00 == 0xfc00 {
        return Err(SsrfBlock::UniqueLocalV6);
    }
    // Teredo 2001::/32 and IPv6 documentation 2001:db8::/32 — reserved.
    if seg[0] == 0x2001 && (seg[1] == 0x0000 || seg[1] == 0x0db8) {
        return Err(SsrfBlock::Reserved);
    }
    Ok(())
}

/// Validate the URL scheme. Only `http`/`https`; credentials in the URL are
/// rejected outright (docs/06 §2.1).
pub fn classify_scheme(scheme: &str, has_credentials: bool) -> Result<(), SsrfBlock> {
    if has_credentials {
        return Err(SsrfBlock::BadScheme);
    }
    match scheme {
        "http" | "https" => Ok(()),
        _ => Err(SsrfBlock::BadScheme),
    }
}

/// Validate an explicit port against the allowlist. `None` (scheme default) is
/// always fine (docs/06 §2.1).
pub fn classify_port(port: Option<u16>) -> Result<(), SsrfBlock> {
    match port {
        None => Ok(()),
        Some(p) if ALLOWED_PORTS.contains(&p) => Ok(()),
        Some(_) => Err(SsrfBlock::BadPort),
    }
}

/// Host-name denylist, applied on top of the IP checks: an internal name that
/// happens to resolve publicly should still not be fetchable (docs/06 §2.2).
/// `host` is the URL host, lowercased by the caller or here.
pub fn classify_host(host: &str) -> Result<(), SsrfBlock> {
    let h = host.trim().trim_end_matches('.').to_ascii_lowercase();

    if h.is_empty() {
        return Err(SsrfBlock::BlockedHost);
    }
    // A bare single-label host (no dot) — e.g. `intranet`, `db` — is never a
    // legitimate public backlink source.
    if !h.contains('.') && h.parse::<IpAddr>().is_err() {
        return Err(SsrfBlock::BlockedHost);
    }
    const BLOCKED_EXACT: &[&str] = &["localhost", "metadata.google.internal"];
    if BLOCKED_EXACT.contains(&h.as_str()) {
        return Err(SsrfBlock::BlockedHost);
    }
    const BLOCKED_SUFFIX: &[&str] = &[".localhost", ".internal", ".local", ".intranet"];
    if BLOCKED_SUFFIX.iter().any(|s| h.ends_with(s)) {
        return Err(SsrfBlock::BlockedHost);
    }
    Ok(())
}

// ─── CDN / shared-front detection (for the C-class footprint signal) ─────────
// A link's source IP is only a useful network-footprint signal when it's the
// site's *own* hosting. Sites behind a big CDN resolve to the CDN's anycast IPs,
// so many unrelated, legitimate sites share one /24 there — counting those as a
// PBN would be wrong. We therefore exclude the dominant CDN ranges before using
// the C-class. IPv4 only (CDN fronting is overwhelmingly v4; v4-mapped v6 is
// unwrapped). The list is intentionally short and easy to extend.
const CDN_V4: &[(Ipv4Addr, u8)] = &[
    // Cloudflare (cloudflare.com/ips-v4)
    (Ipv4Addr::new(173, 245, 48, 0), 20),
    (Ipv4Addr::new(103, 21, 244, 0), 22),
    (Ipv4Addr::new(103, 22, 200, 0), 22),
    (Ipv4Addr::new(103, 31, 4, 0), 22),
    (Ipv4Addr::new(141, 101, 64, 0), 18),
    (Ipv4Addr::new(108, 162, 192, 0), 18),
    (Ipv4Addr::new(190, 93, 240, 0), 20),
    (Ipv4Addr::new(188, 114, 96, 0), 20),
    (Ipv4Addr::new(197, 234, 240, 0), 22),
    (Ipv4Addr::new(198, 41, 128, 0), 17),
    (Ipv4Addr::new(162, 158, 0, 0), 15),
    (Ipv4Addr::new(104, 16, 0, 0), 13),
    (Ipv4Addr::new(104, 24, 0, 0), 14),
    (Ipv4Addr::new(172, 64, 0, 0), 13),
    (Ipv4Addr::new(131, 0, 72, 0), 22),
    // Fastly (main anycast ranges)
    (Ipv4Addr::new(151, 101, 0, 0), 16),
    (Ipv4Addr::new(146, 75, 0, 0), 16),
    (Ipv4Addr::new(199, 232, 0, 0), 16),
    (Ipv4Addr::new(23, 235, 32, 0), 20),
    (Ipv4Addr::new(43, 249, 72, 0), 22),
    (Ipv4Addr::new(104, 156, 80, 0), 20),
];

fn v4_in_cidr(ip: Ipv4Addr, base: Ipv4Addr, prefix: u8) -> bool {
    let mask: u32 = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (u32::from(ip) & mask) == (u32::from(base) & mask)
}

fn as_v4(ip: &str) -> Option<Ipv4Addr> {
    match ip.parse::<IpAddr>().ok()? {
        IpAddr::V4(v) => Some(v),
        IpAddr::V6(v) => v.to_ipv4_mapped(),
    }
}

/// Whether an IP belongs to a known CDN / shared-front range (Cloudflare,
/// Fastly). Such IPs are the CDN's, not the site's host, so they must be
/// excluded from the shared-IP / C-class footprint signal. Non-IPv4 (and
/// unparseable) addresses return false.
pub fn is_cdn_ip(ip: &str) -> bool {
    match as_v4(ip) {
        Some(v4) => CDN_V4
            .iter()
            .any(|&(base, prefix)| v4_in_cidr(v4, base, prefix)),
        None => false,
    }
}

/// The "C-class" of an IP — its /24 (first three octets, e.g. "93.184.216").
/// Two sources on the same C-class likely share hosting. `None` for non-IPv4 or
/// unparseable input.
pub fn c_class(ip: &str) -> Option<String> {
    let o = as_v4(ip)?.octets();
    Some(format!("{}.{}.{}", o[0], o[1], o[2]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn detects_cdn_ranges() {
        assert!(is_cdn_ip("104.16.1.1"), "Cloudflare 104.16.0.0/13");
        assert!(is_cdn_ip("172.67.1.1"), "Cloudflare 172.64.0.0/13");
        assert!(is_cdn_ip("151.101.1.1"), "Fastly 151.101.0.0/16");
        assert!(!is_cdn_ip("93.184.216.34"), "example.com is not a CDN");
        assert!(!is_cdn_ip("8.8.8.8"));
        assert!(!is_cdn_ip("not-an-ip"));
    }

    #[test]
    fn c_class_is_the_first_three_octets() {
        assert_eq!(c_class("93.184.216.34").as_deref(), Some("93.184.216"));
        assert_eq!(c_class("93.184.216.9").as_deref(), Some("93.184.216"));
        assert_eq!(c_class("93.184.217.9").as_deref(), Some("93.184.217"));
        assert_eq!(
            c_class("::ffff:93.184.216.34").as_deref(),
            Some("93.184.216")
        );
        assert_eq!(c_class("garbage"), None);
    }

    #[test]
    fn blocks_the_metadata_endpoints() {
        assert_eq!(classify_ip(ip("169.254.169.254")), Err(SsrfBlock::Metadata));
        assert_eq!(classify_ip(ip("100.100.100.200")), Err(SsrfBlock::Metadata));
    }

    #[test]
    fn blocks_loopback_v4_and_v6() {
        assert_eq!(classify_ip(ip("127.0.0.1")), Err(SsrfBlock::Loopback));
        assert_eq!(classify_ip(ip("127.99.1.2")), Err(SsrfBlock::Loopback));
        assert_eq!(classify_ip(ip("::1")), Err(SsrfBlock::Loopback));
    }

    #[test]
    fn blocks_private_ranges() {
        for a in [
            "10.0.0.1",
            "10.255.255.255",
            "172.16.0.1",
            "172.31.255.1",
            "192.168.1.1",
        ] {
            assert_eq!(
                classify_ip(ip(a)),
                Err(SsrfBlock::Private),
                "{a} should be private"
            );
        }
        // Just outside 172.16/12 is public.
        assert_eq!(classify_ip(ip("172.32.0.1")), Ok(()));
    }

    #[test]
    fn blocks_link_local_and_cgnat_and_reserved() {
        assert_eq!(classify_ip(ip("169.254.1.1")), Err(SsrfBlock::LinkLocal));
        assert_eq!(classify_ip(ip("100.64.0.1")), Err(SsrfBlock::Cgnat));
        assert_eq!(classify_ip(ip("100.127.255.255")), Err(SsrfBlock::Cgnat));
        assert_eq!(classify_ip(ip("192.0.2.1")), Err(SsrfBlock::Reserved));
        assert_eq!(classify_ip(ip("198.18.0.1")), Err(SsrfBlock::Reserved));
        assert_eq!(classify_ip(ip("240.0.0.1")), Err(SsrfBlock::Reserved));
        // 100.128.x is NOT cgnat (outside /10).
        assert_eq!(classify_ip(ip("100.128.0.1")), Ok(()));
    }

    #[test]
    fn blocks_unspecified_and_broadcast_and_multicast() {
        assert_eq!(classify_ip(ip("0.0.0.0")), Err(SsrfBlock::Unspecified));
        assert_eq!(classify_ip(ip("0.1.2.3")), Err(SsrfBlock::Unspecified));
        // 255.255.255.255 is caught by the broadcast check, which runs before
        // the 240/4 reserved rule — the block matters, the exact label is
        // whichever rule fires first.
        assert_eq!(
            classify_ip(ip("255.255.255.255")),
            Err(SsrfBlock::Broadcast)
        );
        assert_eq!(classify_ip(ip("224.0.0.1")), Err(SsrfBlock::Multicast));
    }

    #[test]
    fn v4_mapped_v6_is_unwrapped_and_rechecked() {
        // The classic bypass: ::ffff:127.0.0.1 is loopback in disguise.
        assert_eq!(
            classify_ip(ip("::ffff:127.0.0.1")),
            Err(SsrfBlock::Loopback)
        );
        assert_eq!(
            classify_ip(ip("::ffff:169.254.169.254")),
            Err(SsrfBlock::Metadata)
        );
        assert_eq!(classify_ip(ip("::ffff:10.0.0.1")), Err(SsrfBlock::Private));
        // A mapped PUBLIC address is still fine.
        assert_eq!(classify_ip(ip("::ffff:93.184.216.34")), Ok(()));
    }

    #[test]
    fn six_to_four_embedded_v4_is_rechecked() {
        // 2002:7f00:0001:: embeds 127.0.0.1.
        assert_eq!(classify_ip(ip("2002:7f00:1::")), Err(SsrfBlock::Reserved));
        // 2002: with a public embedded v4 is allowed.
        assert_eq!(classify_ip(ip("2002:5db8:d822::")), Ok(()));
    }

    #[test]
    fn blocks_v6_local_families() {
        assert_eq!(classify_ip(ip("fe80::1")), Err(SsrfBlock::LinkLocal));
        assert_eq!(classify_ip(ip("fc00::1")), Err(SsrfBlock::UniqueLocalV6));
        assert_eq!(classify_ip(ip("fd00::1")), Err(SsrfBlock::UniqueLocalV6));
        assert_eq!(classify_ip(ip("2001:db8::1")), Err(SsrfBlock::Reserved));
        assert_eq!(classify_ip(ip("ff02::1")), Err(SsrfBlock::Multicast));
        assert_eq!(classify_ip(ip("::")), Err(SsrfBlock::Unspecified));
    }

    #[test]
    fn allows_genuine_public_addresses() {
        assert_eq!(classify_ip(ip("93.184.216.34")), Ok(())); // example.com
        assert_eq!(classify_ip(ip("8.8.8.8")), Ok(()));
        assert_eq!(classify_ip(ip("1.1.1.1")), Ok(()));
        assert_eq!(
            classify_ip(ip("2606:2800:220:1:248:1893:25c8:1946")),
            Ok(())
        );
    }

    #[test]
    fn scheme_and_credentials() {
        assert_eq!(classify_scheme("https", false), Ok(()));
        assert_eq!(classify_scheme("http", false), Ok(()));
        assert_eq!(classify_scheme("file", false), Err(SsrfBlock::BadScheme));
        assert_eq!(classify_scheme("gopher", false), Err(SsrfBlock::BadScheme));
        // Credentials in the URL are rejected even on https.
        assert_eq!(classify_scheme("https", true), Err(SsrfBlock::BadScheme));
    }

    #[test]
    fn ports() {
        assert_eq!(classify_port(None), Ok(()));
        assert_eq!(classify_port(Some(80)), Ok(()));
        assert_eq!(classify_port(Some(443)), Ok(()));
        assert_eq!(classify_port(Some(22)), Err(SsrfBlock::BadPort));
        assert_eq!(classify_port(Some(8080)), Err(SsrfBlock::BadPort));
    }

    #[test]
    fn host_denylist() {
        assert_eq!(classify_host("localhost"), Err(SsrfBlock::BlockedHost));
        assert_eq!(classify_host("LOCALHOST."), Err(SsrfBlock::BlockedHost));
        assert_eq!(classify_host("db.internal"), Err(SsrfBlock::BlockedHost));
        assert_eq!(classify_host("api.local"), Err(SsrfBlock::BlockedHost));
        assert_eq!(
            classify_host("metadata.google.internal"),
            Err(SsrfBlock::BlockedHost)
        );
        assert_eq!(classify_host("intranet"), Err(SsrfBlock::BlockedHost)); // bare label
        assert_eq!(classify_host(""), Err(SsrfBlock::BlockedHost));
        // Legitimate public hosts pass.
        assert_eq!(classify_host("example.com"), Ok(()));
        assert_eq!(classify_host("blog.example.co.uk"), Ok(()));
    }
}
