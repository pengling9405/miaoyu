import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "~/components/ui/sidebar";
import { cn } from "~/lib/utils";
import type { ReactNode } from "react";
import { Link, useRouterState } from "@tanstack/react-router";

export type Route = {
  id: string;
  title: string;
  icon?: ReactNode;
  link: string;
  subs?: {
    title: string;
    link: string;
    icon?: ReactNode;
  }[];
};

export default function DashboardNavigation({ routes }: { routes: Route[] }) {
  const { state } = useSidebar();
  const isCollapsed = state === "collapsed";
  const routerState = useRouterState();
  const pathname = routerState.location.pathname;

  const normalizePath = (path: string) => {
    if (!path) return "/";
    if (path === "/") return "/";
    return path.replace(/\/+$/, "") || "/";
  };

  const currentPath = normalizePath(pathname);

  const isPathActive = (target: string | undefined) => {
    if (!target) return false;
    const normalizedTarget = normalizePath(target);
    if (normalizedTarget === "/") {
      return currentPath === "/";
    }
    return (
      currentPath === normalizedTarget ||
      currentPath.startsWith(`${normalizedTarget}/`)
    );
  };

  return (
    <SidebarMenu>
      {routes.map((route) => {
        const isActive = isPathActive(route.link);

        return (
          <SidebarMenuItem key={route.id}>
            <SidebarMenuButton tooltip={route.title} asChild>
              <Link
                to={route.link}
                className={cn(
                  "flex items-center rounded-lg px-2 transition-colors",
                  isActive
                    ? "bg-sidebar-muted text-foreground"
                    : "text-muted-foreground hover:bg-sidebar-muted hover:text-foreground",
                  isCollapsed && "justify-center",
                )}
              >
                {route.icon}
                {!isCollapsed && (
                  <span className="ml-2 text-sm font-medium">
                    {route.title}
                  </span>
                )}
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        );
      })}
    </SidebarMenu>
  );
}
