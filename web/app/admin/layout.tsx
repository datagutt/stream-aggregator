import type { ReactNode } from "react";

import { getAdminKey } from "@/lib/auth";
import { LoginForm } from "@/components/admin/login-form";
import { Sidebar } from "@/components/admin/sidebar";

export default async function AdminLayout({ children }: { children: ReactNode }) {
  const key = await getAdminKey();

  if (!key) return <LoginForm />;

  return (
    <div className="flex min-h-dvh">
      <Sidebar />
      <main className="flex-1 overflow-x-hidden px-[clamp(16px,3vw,40px)] py-8">
        {children}
      </main>
    </div>
  );
}
