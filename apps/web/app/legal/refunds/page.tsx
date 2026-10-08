import type { Metadata } from "next";
import { ProseShell, ProseFooter } from "../../prose-chrome";

export const metadata: Metadata = { title: "Refund & Cancellation Policy" };

export default function RefundsPage() {
  return (
    <ProseShell>
      <h1>Refund &amp; Cancellation Policy</h1>
      <p className="prose-updated">Last updated: 8 October 2026</p>

      <p>
        This policy explains how trials, billing, cancellation and refunds work for LinkPulz, operated
        by MeedWise. It complements our <a href="/legal/terms">Terms of Service</a>.
      </p>

      <h2>Free trial</h2>
      <p>
        Paid plans start with a 10-day free trial. No credit card is required to begin the trial, and
        it ends automatically — you are only charged if you choose to subscribe.
      </p>

      <h2>Billing</h2>
      <p>
        Paid subscriptions are billed in advance on a recurring basis (monthly or annually, depending
        on the plan you choose) through our payment processor, Stripe. Prices shown on our{" "}
        <a href="/pricing">pricing page</a> include any applicable VAT.
      </p>

      <h2>Cancellation</h2>
      <p>
        You can cancel at any time from your billing settings. Your plan stays active until the end of
        the current billing period, and you are not charged again after that. We do not provide
        pro-rata refunds for the unused part of a period.
      </p>

      <h2>Refunds</h2>
      <p>
        Fees already paid are non-refundable except where a refund is required by applicable law (see
        the consumer section below). If you believe you were charged in error, contact us at{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a> and we will review it in good
        faith.
      </p>

      <h2>Consumers — right of withdrawal (EU)</h2>
      <p>
        If you subscribe as a consumer (outside any trade, business or profession), you normally have a
        14-day right of withdrawal under EU and French consumer law. Because LinkPulz is a digital
        service supplied immediately, by starting to use a paid plan during that period you expressly
        request immediate performance and acknowledge that your right of withdrawal is lost once the
        service has been fully performed. This section does not apply to business customers.
      </p>

      <h2>Contact</h2>
      <p>
        Questions about billing, cancellation or a refund:{" "}
        <a href="mailto:contact@linkpulz.com">contact@linkpulz.com</a>.
      </p>

      <ProseFooter />
    </ProseShell>
  );
}
