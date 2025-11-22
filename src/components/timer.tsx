import { useTimer } from "~/hooks/use-timer";

export function Timer() {
	const { time } = useTimer({ autoStart: true });
	const formatTime = (t: number) => {
		const minutes = String(Math.floor(t / 60)).padStart(2, "0");
		const seconds = String(t % 60).padStart(2, "0");
		return `${minutes}:${seconds}`;
	};
	return <div className="text-xs w-8">{formatTime(time)}</div>;
}
