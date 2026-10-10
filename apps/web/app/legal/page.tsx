import type { Metadata } from "next";

export const dynamic = "force-static"; // public content → prerender + edge cache (pairs with the static marketing CSP in middleware.ts)
import { ProseShell, ProseFooter } from "../prose-chrome";

export const metadata: Metadata = { title: "Legal" };

export default function LegalIndex() {
  return (
    <ProseShell>
      <h1>Legal</h1>
      <p className="prose-updated">The policies that govern your use of LinkPulz.</p>
      <ul className="prose-cards">
        <li>
          <a href="/legal/privacy">
            <strong>Privacy Policy</strong>
            <span>What data we collect, why, and your rights over it.</span>
          </a>
        </li>
        <li>
          <a href="/legal/terms">
            <strong>Terms of Service</strong>
            <span>The agreement between you and LinkPulz.</span>
          </a>
        </li>
        <li>
          <a href="/legal/cookies">
            <strong>Cookie Policy</strong>
            <span>The cookies we use and how to control them.</span>
          </a>
        </li>
        <li>
          <a href="/legal/subprocessors">
            <strong>Sub-processors</strong>
            <span>The providers that process data to run the Service.</span>
          </a>
        </li>
        <li>
          <a href="/legal/dpa">
            <strong>Data Processing Agreement</strong>
            <span>How we process data on behalf of business customers.</span>
          </a>
        </li>
        <li>
          <a href="/legal/refunds">
            <strong>Refund &amp; Cancellation</strong>
            <span>Trials, billing, cancellation and refunds.</span>
          </a>
        </li>
        <li>
          <a href="/legal/notice">
            <strong>Legal Notice</strong>
            <span>Company identity and hosting information.</span>
          </a>
        </li>
      </ul>
      <ProseFooter />
    </ProseShell>
  );
}
