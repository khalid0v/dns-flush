interface PageHeaderProps {
  title: string;
  sub: string;
}

export function PageHeader({ title, sub }: PageHeaderProps) {
  return (
    <div className="mb-4">
      <h1 className="text-xl font-semibold tracking-tight sm:text-2xl">{title}</h1>
      <p className="mt-1 text-sm text-muted-foreground">{sub}</p>
    </div>
  );
}
