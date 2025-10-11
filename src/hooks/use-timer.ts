import { useCallback, useEffect, useRef, useState } from "react";

type UseTimerOptions = {
	autoStart?: boolean;
	tickMs?: number;
};

type UseTimerReturn = {
	time: number;
	isRunning: boolean;
	start: () => void;
	stop: () => void;
	reset: () => void;
};

const DEFAULT_INTERVAL = 1000;

export function useTimer(options: UseTimerOptions = {}): UseTimerReturn {
	const { autoStart = false, tickMs = DEFAULT_INTERVAL } = options;
	const [time, setTime] = useState(0);
	const [isRunning, setIsRunning] = useState(autoStart);
	const intervalRef = useRef<number | null>(null);

	const clearTimer = useCallback(() => {
		if (intervalRef.current !== null) {
			window.clearInterval(intervalRef.current);
			intervalRef.current = null;
		}
	}, []);

	const start = useCallback(() => {
		setIsRunning(true);
	}, []);

	const stop = useCallback(() => {
		setIsRunning(false);
	}, []);

	const reset = useCallback(() => {
		setTime(0);
	}, []);

	useEffect(() => {
		if (!isRunning) {
			clearTimer();
			return;
		}

		clearTimer();
		intervalRef.current = window.setInterval(() => {
			setTime((prev) => prev + 1);
		}, tickMs);

		return () => {
			clearTimer();
		};
	}, [isRunning, tickMs, clearTimer]);

	useEffect(() => {
		if (!autoStart) {
			return;
		}

		start();
	}, [autoStart, start]);

	useEffect(
		() => () => {
			clearTimer();
		},
		[clearTimer],
	);

	return { time, isRunning, start, stop, reset };
}
