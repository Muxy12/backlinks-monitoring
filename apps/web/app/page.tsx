import { HOMEPAGE_HTML } from "./homepage-html";

// The marketing homepage (§48). Ported 1:1 from the approved design
// (docs/homepage-source.html), rebranded to LinkPulz. Static, self-authored
// markup with no user data, rendered by a Server Component — so it ships zero
// client JavaScript (docs/14 §4.1).
export default function HomePage() {
  return <div dangerouslySetInnerHTML={{ __html: HOMEPAGE_HTML }} />;
}
