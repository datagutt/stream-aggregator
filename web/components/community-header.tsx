import Link from "next/link";
import type { Community } from "@/lib/api-types";
import { ThemeToggle } from "./theme-toggle";

export function CommunityHeader({
  community,
  showAdminLink,
}: {
  community: Community;
  showAdminLink: boolean;
}) {
  return (
    <header className="border-b border-border bg-background/85 sticky top-0 z-10 backdrop-blur">
      <div className="mx-auto flex max-w-[1600px] items-center gap-4 px-[clamp(16px,4vw,48px)] py-4">
        <Link href={`/c/${community.slug}`} className="flex min-w-0 items-center gap-3">
          {community.logoUrl ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img
              src={community.logoUrl}
              alt={community.name}
              className="size-7 rounded-md object-cover"
            />
          ) : (
            <span
              aria-hidden
              className="size-7 rounded-md"
              style={{ background: "var(--color-brand)" }}
            />
          )}
          <div className="min-w-0">
            <p className="truncate text-base font-semibold tracking-tight text-foreground sm:text-lg">
              {community.name}
            </p>
            {community.tagline && (
              <p className="truncate text-xs text-foreground-muted">{community.tagline}</p>
            )}
          </div>
        </Link>

        <div className="ml-auto flex items-center gap-2">
          {community.aboutMd && (
            <Link
              href={`/c/${community.slug}/about`}
              className="text-foreground-muted hover:text-foreground hidden text-sm sm:inline"
            >
              About
            </Link>
          )}
          <Link
            href={`/c/${community.slug}/all`}
            className="text-foreground-muted hover:text-foreground hidden text-sm sm:inline"
          >
            All streamers
          </Link>
          {showAdminLink && (
            <Link
              href="/admin"
              className="text-foreground-muted hover:text-foreground hidden text-sm sm:inline"
            >
              Admin
            </Link>
          )}
          <ThemeToggle />
        </div>
      </div>
    </header>
  );
}
