import type { Metadata } from "next";

// /register is a utility page, the sign-up form, or (while sign-ups are closed)
// a waitlist capture. Keep it out of search indexes. (robots.txt also disallows
// crawling /register; this noindex is the belt-and-braces directive.)
export const metadata: Metadata = {
  robots: { index: false, follow: false },
};

export default function RegisterLayout({ children }: { children: React.ReactNode }) {
  return children;
}
