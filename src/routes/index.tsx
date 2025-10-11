import { createFileRoute } from "@tanstack/react-router";
import { AudioPanel } from "~/components/audio-panel";

export const Route = createFileRoute("/")({
	component: RouteComponent,
});

function RouteComponent() {
	return <AudioPanel />;
}
