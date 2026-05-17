import { listPlatforms } from "@/lib/api";
import { CommunityEditor } from "@/components/admin/community-editor";

export default async function NewCommunityPage() {
  const platforms = await listPlatforms({ revalidate: 60 });
  return (
    <div className="space-y-6">
      <header className="space-y-1">
        <p className="text-foreground-dim font-mono text-xs uppercase tracking-wider">
          New community
        </p>
        <h1 className="text-2xl font-semibold tracking-tight">
          Create a community
        </h1>
      </header>

      <CommunityEditor platformIds={platforms.map((p) => p.id)} />
    </div>
  );
}
