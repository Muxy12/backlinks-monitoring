// Glossary data (programmatic SEO). Each entry renders one static page at
// /glossary/<slug> via the shared [slug] template, plus a row on the /glossary
// hub. Content is definitional and factual — no invented statistics or claims.
// Terms are chosen to NOT cannibalize the existing marketing pages (backlink
// monitoring, lost/broken/verify backlinks, backlink profile/audit/…): these are
// upstream, definitional queries that feed into that cluster. `cluster` links the
// term to the most relevant existing page; `related` links to sibling terms.

export type GlossarySection = { h: string; body: string[] };

export type GlossaryTerm = {
  slug: string;
  term: string; // display name, e.g. "Backlink"
  h1: string; // page headline, usually a question
  title: string; // <title> (the shell appends nothing; keep it whole)
  description: string; // meta description, ~150–160 chars
  definition: string[]; // lead definition paragraph(s)
  sections: GlossarySection[]; // deeper explanation
  related: string[]; // other term slugs
  cluster?: { href: string; label: string }; // most relevant marketing page
};

export const GLOSSARY: GlossaryTerm[] = [
  {
    slug: "what-is-a-backlink",
    term: "Backlink",
    h1: "What is a backlink?",
    title: "What Is a Backlink? Definition & Examples | LinkPulz",
    description:
      "A backlink is a link from one website to another. Learn what backlinks are, why they matter for SEO, and the parts that make up a single backlink.",
    definition: [
      "A backlink is a link on one website that points to a page on another website. If a page on example.com links to your site, that link is a backlink for you and an outbound link for example.com. Search engines treat backlinks as signals of relevance and trust, which is why they matter for SEO.",
    ],
    sections: [
      {
        h: "What a single backlink is made of",
        body: [
          "Every backlink has a few parts worth knowing: the <b>source URL</b> (the page the link sits on) and its <b>referring domain</b>, the <b>target URL</b> (the page it points to, on your site), the <b>anchor text</b> (the clickable words), and its <b>rel attributes</b>, which decide whether the link is dofollow or nofollow.",
          "Those parts are exactly what changes over time: a link can be removed, its anchor edited, its rel switched to nofollow, or the page it sits on can start returning an error. Tracking them is what backlink monitoring is for.",
        ],
      },
      {
        h: "Why backlinks matter for SEO",
        body: [
          "A relevant backlink from a trusted site can pass authority to the page it points to and send referral traffic. Not all backlinks are equal: a link from a topically relevant, reputable page is worth more than one from a low-quality or unrelated site, and a nofollow link is treated differently from a dofollow one.",
          "Because backlinks are earned (or paid for) and can disappear without warning, knowing which links you have — and being told when one changes — is as important as building them in the first place.",
        ],
      },
    ],
    related: ["referring-domain", "anchor-text", "dofollow-vs-nofollow", "link-equity"],
    cluster: { href: "/", label: "backlink monitoring" },
  },
  {
    slug: "dofollow-vs-nofollow",
    term: "Dofollow vs nofollow links",
    h1: "What is the difference between dofollow and nofollow links?",
    title: "Dofollow vs Nofollow Links: What's the Difference? | LinkPulz",
    description:
      "A dofollow link passes ranking signals; a nofollow link tells search engines not to. Learn the difference, the rel values involved, and why changes matter.",
    definition: [
      "A <b>dofollow</b> link is an ordinary link that can pass ranking signals to the page it points to. A <b>nofollow</b> link carries a <code>rel=\"nofollow\"</code> attribute that tells search engines not to pass those signals. Both are real links a visitor can click; the difference is the instruction they give to search engines.",
    ],
    sections: [
      {
        h: "The rel attribute decides",
        body: [
          'Whether a link is dofollow or nofollow comes down to its <code>rel</code> attribute. No <code>rel</code> (or a <code>rel</code> without "nofollow") means dofollow. <code>rel="nofollow"</code>, and the related <code>rel="sponsored"</code> and <code>rel="ugc"</code> values, all tell search engines to discount the link to different degrees.',
          "Since 2019 Google treats nofollow, sponsored and ugc as hints rather than strict directives, but the practical point for link building is unchanged: a dofollow link from a relevant page is generally the more valuable outcome.",
        ],
      },
      {
        h: "Why a dofollow→nofollow change matters",
        body: [
          "A publisher can change a link's rel after it is published — for example switching a once-dofollow link to nofollow. The link still exists, so a simple presence check looks fine, but its SEO value has changed. Catching that shift is one of the reasons to monitor attributes, not just whether a link is still there.",
        ],
      },
    ],
    related: ["sponsored-ugc-links", "what-is-a-backlink", "link-equity", "anchor-text"],
    cluster: { href: "/verify-backlinks", label: "verify a backlink" },
  },
  {
    slug: "referring-domain",
    term: "Referring domain",
    h1: "What is a referring domain?",
    title: "What Is a Referring Domain? Definition & Why It Matters | LinkPulz",
    description:
      "A referring domain is a unique website that links to you. Learn how referring domains differ from backlinks and why the count matters more than raw links.",
    definition: [
      "A referring domain is a unique website that has at least one backlink pointing to your site. If a single blog links to you from five different articles, that is five backlinks but only one referring domain.",
    ],
    sections: [
      {
        h: "Referring domains vs backlinks",
        body: [
          "Backlinks count every individual link; referring domains count the distinct sites those links come from. The distinction matters because a hundred links from one site usually say less about your authority than links from a hundred different relevant sites.",
          "When you analyze a backlink profile, the spread of referring domains — how many, how relevant, how reputable — is often a better health signal than the raw backlink total.",
        ],
      },
      {
        h: "Referring domains in competitor analysis",
        body: [
          "Referring domains are also the unit of competitor gap analysis: by comparing the domains that link to a competitor with the domains that link to you, you can surface relevant sites that already link to rivals but not yet to you — a ready-made outreach shortlist.",
        ],
      },
    ],
    related: ["what-is-a-backlink", "link-building", "anchor-text", "toxic-backlinks"],
    cluster: { href: "/backlink-profile", label: "analyze a backlink profile" },
  },
  {
    slug: "anchor-text",
    term: "Anchor text",
    h1: "What is anchor text?",
    title: "What Is Anchor Text? Types & SEO Best Practices | LinkPulz",
    description:
      "Anchor text is the clickable words of a link. Learn the common anchor text types, why a natural mix matters, and why anchor changes are worth monitoring.",
    definition: [
      "Anchor text is the visible, clickable text of a hyperlink — usually styled and underlined. For a backlink, the anchor text is one signal search engines use to understand what the linked page is about.",
    ],
    sections: [
      {
        h: "Common anchor text types",
        body: [
          "Anchors are often grouped into types: <b>branded</b> (your brand or domain name), <b>exact-match</b> or keyword (the exact phrase you want to rank for), <b>partial-match</b>, <b>generic</b> (\"click here\", \"read more\"), and <b>naked URL</b> (the URL itself). Image links have no text anchor, so their alt text plays a similar role.",
          "A natural backlink profile usually has a varied mix. An unnaturally high share of exact-match keyword anchors across many links can look manipulative, so the distribution matters more than any single anchor.",
        ],
      },
      {
        h: "Why anchor changes matter",
        body: [
          "A publisher can edit the anchor text of a link after publication, which changes the signal the link sends. Because the link is still present, a basic check won't flag it — monitoring the anchor itself is what surfaces the change.",
        ],
      },
    ],
    related: ["what-is-a-backlink", "dofollow-vs-nofollow", "referring-domain", "link-equity"],
    cluster: { href: "/backlink-profile", label: "analyze your backlink profile" },
  },
  {
    slug: "link-equity",
    term: "Link equity",
    h1: "What is link equity?",
    title: "What Is Link Equity (Link Juice)? Definition | LinkPulz",
    description:
      "Link equity is the ranking value a link can pass from one page to another. Learn what affects it and why losing a strong link reduces the equity you receive.",
    definition: [
      "Link equity — sometimes called \"link juice\" — is the ranking value that a link can pass from the page it sits on to the page it points to. It is an informal term for a real idea: not every link passes the same amount of value.",
    ],
    sections: [
      {
        h: "What affects how much equity a link passes",
        body: [
          "Several factors influence link equity: the authority and relevance of the source page and domain, whether the link is dofollow or nofollow, the number of other links on the source page, and the relevance of the anchor text and surrounding content. A dofollow link from a relevant, reputable page passes more than a nofollow link from an unrelated one.",
        ],
      },
      {
        h: "Why losing a link costs equity",
        body: [
          "When a backlink is removed, redirected to the wrong place, or switched to nofollow, the equity it used to pass to your page is reduced or gone. That is why catching a lost or changed link early matters: the sooner you know, the sooner you can try to recover it while the change is still recent.",
        ],
      },
    ],
    related: ["what-is-a-backlink", "dofollow-vs-nofollow", "referring-domain", "toxic-backlinks"],
    cluster: { href: "/lost-backlinks", label: "find lost backlinks" },
  },
  {
    slug: "sponsored-ugc-links",
    term: "Sponsored and UGC links",
    h1: "What are sponsored and UGC links?",
    title: "Sponsored vs UGC Links: rel Attributes Explained | LinkPulz",
    description:
      "rel=\"sponsored\" marks paid links; rel=\"ugc\" marks user-generated links. Learn what these rel values mean, when to use them, and how they relate to nofollow.",
    definition: [
      "<code>rel=\"sponsored\"</code> and <code>rel=\"ugc\"</code> are link attributes Google introduced in 2019 to describe <i>why</i> a link is nofollowed. <b>Sponsored</b> marks links that are paid or part of an advertisement; <b>UGC</b> (user-generated content) marks links created by users, such as blog comments or forum posts.",
    ],
    sections: [
      {
        h: "How they relate to nofollow",
        body: [
          "All three — nofollow, sponsored and ugc — tell search engines to treat the link with caution. You can use them together (for example <code>rel=\"sponsored nofollow\"</code>). Sponsored and ugc simply add context about the link's origin, which Google asked publishers to provide for paid and user-created links.",
          "For the site earning the link, a paid or user-generated link marked this way typically passes little or no ranking value by design.",
        ],
      },
      {
        h: "Why it matters when you buy links",
        body: [
          "If you pay for a placement, the publisher may mark it sponsored or nofollow — which changes what you actually get for your spend. Knowing the rel attributes on your paid links, and being alerted if they change, helps you verify that a placement still matches what you agreed to.",
        ],
      },
    ],
    related: ["dofollow-vs-nofollow", "what-is-a-backlink", "link-equity", "toxic-backlinks"],
    cluster: { href: "/verify-backlinks", label: "check a link's rel attributes" },
  },
  {
    slug: "toxic-backlinks",
    term: "Toxic backlinks",
    h1: "What are toxic backlinks?",
    title: "What Are Toxic Backlinks? Definition & What to Do | LinkPulz",
    description:
      "A toxic backlink is a low-quality or spammy link that could harm your site. Learn what makes a link toxic and a measured, modern view of what to do about it.",
    definition: [
      "A toxic backlink is a low-quality, spammy or unnatural link that a site owner worries could harm their rankings. Typical examples are links from link farms, unrelated foreign-language spam sites, or obvious paid-link networks.",
    ],
    sections: [
      {
        h: "A measured, modern view",
        body: [
          "\"Toxicity\" is not a metric Google publishes; it is an interpretation applied by third-party tools, and their scores often disagree. Google has repeatedly said its systems ignore most low-quality links automatically, so a few spammy links are usually not worth worrying about.",
          "That is why LinkPulz does not assign a \"toxic score\": it would be an invented number dressed up as fact. The useful work is observing your real links and their changes, not labelling them with a confidence the data doesn't support.",
        ],
      },
      {
        h: "When links genuinely need attention",
        body: [
          "The clearer signal is a sudden, unexplained surge of unrelated links, which can point to a negative-SEO attempt or a hacked page. In the rare cases where manual action is warranted, Google's disavow tool exists — but it is a last resort, not routine hygiene.",
        ],
      },
    ],
    related: ["referring-domain", "link-equity", "what-is-a-backlink", "link-building"],
    cluster: { href: "/backlink-health-check", label: "run a backlink health check" },
  },
  {
    slug: "link-building",
    term: "Link building",
    h1: "What is link building?",
    title: "What Is Link Building? Definition & Common Methods | LinkPulz",
    description:
      "Link building is the practice of earning backlinks from other sites. Learn the common methods and why monitoring is the step most teams skip after building.",
    definition: [
      "Link building is the practice of earning backlinks from other websites to your own, in order to grow your site's authority and referral traffic. It covers everything from earning editorial mentions to active outreach and digital PR.",
    ],
    sections: [
      {
        h: "Common link-building methods",
        body: [
          "Typical approaches include creating content worth citing, guest posting, digital PR, broken-link building, unlinked-brand-mention outreach, and partnerships. Some teams also buy placements; wherever a link comes from, it is an asset that took time or budget to acquire.",
        ],
      },
      {
        h: "The step most teams skip: monitoring",
        body: [
          "Building a link is the start, not the end. Publishers remove links, edit articles, migrate sites and change rel attributes — so links you earned or paid for can quietly disappear or lose value. Monitoring your backlinks closes that loop: it verifies the links you built are still live, still dofollow and still pointing where they should.",
        ],
      },
    ],
    related: ["what-is-a-backlink", "referring-domain", "anchor-text", "link-equity"],
    cluster: { href: "/backlink-management", label: "manage your backlinks" },
  },
];

export const GLOSSARY_BY_SLUG: Record<string, GlossaryTerm> = Object.fromEntries(
  GLOSSARY.map((t) => [t.slug, t]),
);
