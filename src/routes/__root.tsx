/// <reference types="vite/client" />

import type { QueryClient } from "@tanstack/react-query";
import {
	createRootRouteWithContext,
	HeadContent,
	Outlet,
	Scripts,
	useRouterState,
} from "@tanstack/react-router";
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
	const routerState = useRouterState();
	const pathname = routerState.location.pathname;
	const bodyClassName =
		pathname === "/" || pathname === "/feedback"
			? "bg-transparent"
			: "bg-background";

	// 确保 body className 在客户端正确更新（解决 SSR/hydration 问题）
	useEffect(() => {
		if (typeof document !== "undefined") {
			document.body.className = bodyClassName;
		}
	}, [bodyClassName]);

	return (
		<RootDocument bodyClassName={bodyClassName}>
			<Outlet />
		</RootDocument>
	);
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
