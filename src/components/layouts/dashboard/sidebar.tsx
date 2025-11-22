import { getVersion } from "@tauri-apps/api/app";
import { Cpu, Home, PanelLeftIcon, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "~/components/ui/button";
import {
	Sidebar,
	SidebarContent,
	SidebarFooter,
	SidebarHeader,
	useSidebar,
} from "~/components/ui/sidebar";
import {
	Tooltip,
	TooltipContent,
	TooltipTrigger,
} from "~/components/ui/tooltip";
import { cn } from "~/lib/utils";
import { Logo } from "./logo";
import type { Route } from "./nav-main";
import DashboardNavigation from "./nav-main";

const dashboardRoutes: Route[] = [
	{
		id: "home",
		title: "首页",
		icon: <Home className="size-4" />,
		link: "/",
	},
	{
		id: "models",
		title: "模型",
		icon: <Cpu className="size-4" />,
		link: "/models",
	},
	{
		id: "settings",
		title: "设置",
		icon: <Settings className="size-4" />,
		link: "/settings",
	},
];

export function DashboardSidebar() {
	const { state, toggleSidebar } = useSidebar();
	const isCollapsed = state === "collapsed";
	const [appVersion, setAppVersion] = useState<string | null>(null);

	useEffect(() => {
		let mounted = true;

		const fetchVersion = async () => {
			try {
				const version = await getVersion();
				if (mounted) {
					setAppVersion(version);
				}
			} catch (error) {
				console.debug("Failed to resolve app version", error);
			}
		};

		void fetchVersion();

		return () => {
			mounted = false;
		};
	}, []);

	return (
		<Sidebar variant="inset" collapsible="icon">
			{isCollapsed ? (
				<SidebarHeader className="flex items-center justify-center group">
					<Logo className="size-8 group-hover:hidden" />
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								data-sidebar="trigger"
								data-slot="sidebar-trigger"
								variant="ghost"
								size="icon"
								className="size-8 hidden group-hover:flex"
								onClick={() => {
									toggleSidebar();
								}}
							>
								<PanelLeftIcon />
								<span className="sr-only">打开侧边栏</span>
							</Button>
						</TooltipTrigger>
						<TooltipContent side="right">打开侧边栏</TooltipContent>
					</Tooltip>
				</SidebarHeader>
			) : (
				<SidebarHeader className="flex flex-row items-center justify-between">
					<a href="#" className="flex items-center gap-2">
						<Logo className="h-8 w-8" />
						<span className="font-semibold text-black dark:text-white">
							妙语
						</span>
					</a>
					<Tooltip>
						<TooltipTrigger asChild>
							<Button
								data-sidebar="trigger"
								data-slot="sidebar-trigger"
								variant="ghost"
								size="icon"
								className="size-8"
								onClick={() => {
									toggleSidebar();
								}}
							>
								<PanelLeftIcon />
								<span className="sr-only">关闭侧边栏</span>
							</Button>
						</TooltipTrigger>
						<TooltipContent side="right">关闭侧边栏</TooltipContent>
					</Tooltip>
				</SidebarHeader>
			)}
			<SidebarContent className="gap-4 px-2 py-4">
				<DashboardNavigation routes={dashboardRoutes} />
			</SidebarContent>
			<SidebarFooter className="group-data-[state=collapsed]:px-0">
				<div
					className={cn(
						"flex w-full gap-0.5 p-2 pb-0 text-xs transition-colors text-muted-foreground",
						"group-data-[state=collapsed]:items-center group-data-[state=collapsed]:px-2",
					)}
				>
					<span
						className={"tracking-wide group-data-[state=collapsed]:sr-only"}
					>
						当前版本
					</span>
					<span>v{appVersion ?? "--"}</span>
				</div>
			</SidebarFooter>
		</Sidebar>
	);
}
