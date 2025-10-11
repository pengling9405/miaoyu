export function NotFound({ children }: { children?: React.ReactNode }) {
	return (
		<div className="space-y-2 p-2">
			<div className="text-gray-600 dark:text-gray-400">
				{children || <p>页面不存在</p>}
			</div>
			<p className="flex items-center gap-2 flex-wrap">
				<button
					onClick={() => window.history.back()}
					className="bg-emerald-500 text-white px-2 py-1 rounded uppercase font-black text-sm"
				>
					返回
				</button>
			</p>
		</div>
	);
}
