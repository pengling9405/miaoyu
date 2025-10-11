import type { ErrorComponentProps } from "@tanstack/react-router";
import { ErrorComponent, useRouter } from "@tanstack/react-router";

export function ErrorBoundary({ error }: ErrorComponentProps) {
	const router = useRouter();
	return (
		<div className="min-w-0 flex-1 p-4 flex flex-col items-center justify-center gap-6">
			<ErrorComponent error={error} />
			<div className="flex gap-2 items-center flex-wrap">
				<button
					onClick={() => {
						router.invalidate();
					}}
					className={`px-2 py-1 bg-gray-600 dark:bg-gray-700 rounded text-white uppercase font-extrabold`}
				>
					刷新页面
				</button>
			</div>
		</div>
	);
}
