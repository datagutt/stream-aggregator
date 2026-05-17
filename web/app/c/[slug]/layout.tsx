import type { CSSProperties, ReactNode } from "react";
import { notFound } from "next/navigation";
import { cookies } from "next/headers";

import { getCommunity } from "@/lib/communities";
import { ADMIN_COOKIE } from "@/lib/auth";
import { oklchTriplet } from "@/lib/format";
import { CommunityHeader } from "@/components/community-header";

interface Props {
  children: ReactNode;
  params: Promise<{ slug: string }>;
}

export default async function CommunityLayout({ children, params }: Props) {
  const { slug } = await params;
  const community = await getCommunity(slug);
  if (!community) notFound();

  const showAdminLink = (await cookies()).has(ADMIN_COOKIE);

  // Inject the community's brand tokens as CSS variables on a wrapping div.
  // All descendants — header, grid, cards — pick them up. Other surfaces
  // (admin, _not-configured) stay on the global default.
  const brandStyle: CSSProperties & Record<string, string> = {
    ["--color-brand"]: oklchTriplet(community.accent, "0.68 0.16 25"),
    ["--color-brand-contrast"]: oklchTriplet(community.accentContrast, "0.98 0.01 25"),
  };

  return (
    <div
      data-community={community.slug}
      data-default-theme={community.defaultTheme}
      style={brandStyle}
      className="min-h-dvh"
    >
      <CommunityHeader community={community} showAdminLink={showAdminLink} />
      <main className="mx-auto max-w-[1600px] px-[clamp(16px,4vw,48px)] py-6">{children}</main>
    </div>
  );
}
