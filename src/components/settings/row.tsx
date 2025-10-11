type Props = {
	title: string;
	description: string;
	action: React.ReactNode;
};

export function SettingRow({ title, description, action }: Props) {
	return (
		<div className="flex gap-4 items-end justify-between py-4">
			<div className="space-y-1">
				<div className="text-sm font-medium text-foreground">{title}</div>
				<p className="text-xs text-muted-foreground">{description}</p>
			</div>
			<div className="flex items-end justify-end">{action}</div>
		</div>
	);
}
