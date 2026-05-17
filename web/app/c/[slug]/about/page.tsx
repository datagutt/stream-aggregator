import { notFound } from "next/navigation";
import { marked } from "marked";

import { getCommunity } from "@/lib/communities";

interface Props {
  params: Promise<{ slug: string }>;
}

/**
 * /about — renders the community's aboutMd field as HTML. about_md is
 * admin-controlled (set via the admin community editor), so input is trusted.
 */
export default async function CommunityAbout({ params }: Props) {
  const { slug } = await params;
  const community = await getCommunity(slug);
  if (!community) notFound();

  if (!community.aboutMd) {
    return (
      <article className="prose-readable">
        <h1>{community.name}</h1>
        <p className="text-foreground-muted">
          This community hasn&apos;t written an about page yet.
        </p>
      </article>
    );
  }

  const html = await marked.parse(community.aboutMd, { async: true });

  return (
    <article className="prose-readable">
      <h1>{community.name}</h1>
      {/* about_md is admin-only input; we trust the markdown source. */}
      <div dangerouslySetInnerHTML={{ __html: html }} />
    </article>
  );
}
