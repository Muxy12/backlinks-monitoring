import { permanentRedirect } from "next/navigation";

// During the pre-launch period the full homepage lived here while "/" served a
// waitlist. At launch the homepage moved to "/", so /home now permanently
// redirects there, one canonical homepage URL, old links kept alive.
export default function MarketingHomePage() {
  permanentRedirect("/");
}
