import { createFileRoute } from "@tanstack/react-router";
import { AudioWave } from "~/components/audio-wave";
import { MovingLabel } from "~/components/moving-border";
import { Timer } from "~/components/timer";

export const Route = createFileRoute("/recording")({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <div className="flex h-screen w-screen items-center justify-center">
      <MovingLabel
        borderRadius="1.75rem"
        duration={3500}
        className="flex w-full h-full overflow-hidden items-center justify-center gap-2 rounded-full border border-border bg-background shadow-sm"
        containerClassName="w-30 h-8"
      >
        <AudioWave />
        <Timer />
      </MovingLabel>
    </div>
  );
}
