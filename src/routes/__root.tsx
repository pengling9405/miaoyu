/// <reference types="vite/client" />

import type { QueryClient } from "@tanstack/react-query";
import {
	createRootRouteWithContext,
	HeadContent,
	Outlet,
	Scripts,
	useRouter,
	useRouterState,
} from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { ReactNode } from "react";
import { useEffect } from "react";
import appCss from "~/app.css?url";
import { ErrorBoundary } from "~/components/error-boundary";
import { NotFound } from "~/components/not-found";
import { ThemeProvider } from "~/components/theme/provider";

export const Route = createRootRouteWithContext<{
	queryClient: QueryClient;
}>()({
	head: () => ({
		meta: [
			{
				charSet: "utf-8",
			},
			{
				name: "viewport",
				content: "width=device-width, initial-scale=1",
			},
			{
				title: "妙语 - 智能语音输入",
			},
		],
		links: [{ rel: "stylesheet", href: appCss }],
	}),
	notFoundComponent: () => <NotFound />,
	errorComponent: (props) => (
		<RootDocument>
			<ErrorBoundary {...props} />
		</RootDocument>
	),
	component: RootComponent,
});

function RootComponent() {
	const router = useRouter();
	const routerState = useRouterState();
	const pathname = routerState.location.pathname;
	const overlayPrefixes = ["/recording", "/transcribing"];
	const isOverlayRoute =
		pathname === "/notification" ||
		overlayPrefixes.some((prefix) => pathname.startsWith(prefix));
	const bodyClassName = isOverlayRoute ? "bg-transparent" : "bg-background";

	// 确保 body className 在客户端正确更新（解决 SSR/hydration 问题）
	useEffect(() => {
		if (typeof document !== "undefined") {
			document.body.className = bodyClassName;
		}
	}, [bodyClassName]);

	useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}

		let windowLabel: string | null = null;
		try {
			windowLabel = getCurrentWebviewWindow().label;
		} catch {
			windowLabel = null;
		}

		if (windowLabel !== "dashboard") {
			return;
		}

		let cleanup: (() => void) | undefined;
		let isMounted = true;

		const syncPendingNavigation = async () => {
			const pending = await takePendingNavigation();
			if (!pending || !isMounted) {
				return;
			}
			router.navigate({ to: pending as never });
		};

		void syncPendingNavigation();

		listen<{ path?: string }>("navigate", (event) => {
			const targetPath = event.payload?.path ?? "/";
			router.navigate({ to: targetPath as never });
			void takePendingNavigation();
		})
			.then((unlisten) => {
				cleanup = unlisten;
			})
			.catch(() => {
				/* ignore */
			});

		return () => {
			isMounted = false;
			cleanup?.();
		};
	}, [router]);

	return (
		<RootDocument bodyClassName={bodyClassName}>
			<Outlet />
		</RootDocument>
	);
}

async function takePendingNavigation(): Promise<string | null> {
	if (typeof window === "undefined") {
		return null;
	}

	try {
		const path = await invoke<string | null>("take_pending_navigation");
		return path ?? null;
	} catch {
		return null;
	}
}

function RootDocument({
	children,
	bodyClassName,
}: Readonly<{ children: ReactNode; bodyClassName?: string }>) {
	return (
		<html suppressHydrationWarning>
			<head>
				<HeadContent />
			</head>
			<body className={bodyClassName}>
				<ThemeProvider>{children}</ThemeProvider>
				<Scripts />
			</body>
		</html>
	);
}
