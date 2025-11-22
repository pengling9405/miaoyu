import type { PropsWithChildren } from "react";
import { SidebarInset, SidebarProvider } from "~/components/ui/sidebar";
import { DashboardSidebar } from "./sidebar";

export function Dashboard({ children }: PropsWithChildren) {
	return (
		<SidebarProvider>
			<div className="relative flex h-screen w-full">
				<DashboardSidebar />
				<SidebarInset className="flex flex-col py-2 px-6">
					{children}
				</SidebarInset>
			</div>
		</SidebarProvider>
	);
}
