import type { Metadata } from "next";
import { ProseShell, ProseFooter } from "../prose-chrome";

export const metadata: Metadata = { title: "Legal" };

export default function LegalIndex() {
  return (
    <ProseShell>
      <h1>Legal</h1>
      <p className="prose-updated">The policies that govern your use of Backlio.</p>
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
            <span>The agreement between you and Backlio.</span>
          </a>
        </li>
        <li>
          <a href="/legal/cookies">
            <strong>Cookie Policy</strong>
            <span>The cookies we use and how to control them.</span>
          </a>
        </li>
      </ul>
      <ProseFooter />
    </ProseShell>
  );
}
