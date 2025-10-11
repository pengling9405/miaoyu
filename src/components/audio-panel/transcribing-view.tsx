export function TranscribingView() {
	return (
		<div className="bg-background border border-border w-full h-8 rounded-full shadow-sm flex items-center justify-center">
			<div className="flex items-center gap-0.5">
				{Array.from({ length: 9 }, (_, i) => (
					<div
						key={`loading-dot-${i}`}
						className="w-0.5 h-0.5 bg-foreground rounded-full animate-loading"
						style={{
							animationDelay: `${i * 0.2}s`,
						}}
					/>
				))}
			</div>
		</div>
	);
}
