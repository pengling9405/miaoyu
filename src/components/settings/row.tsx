type Props = {
  title: string;
  description: string;
  action: React.ReactNode;
  icon?: React.ReactNode;
};

export function SettingRow({ title, description, action, icon }: Props) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex items-center gap-3">
        {icon && (
          <div className="rounded-2xl border border-border/60 bg-muted/40 p-2 text-muted-foreground">
            {icon}
          </div>
        )}
        <div className="space-y-1">
          <div className="text-sm font-medium text-foreground">{title}</div>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
      </div>
      <div className="flex items-center justify-end">{action}</div>
    </div>
  );
}
