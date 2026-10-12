import type {
  TableHTMLAttributes,
  HTMLAttributes,
  ThHTMLAttributes,
  TdHTMLAttributes,
} from "react";
export function Table({
  className = "",
  ...props
}: TableHTMLAttributes<HTMLTableElement>) {
  return <table {...props} className={`ui-table ${className}`} />;
}
export function TableHeader({
  className = "",
  ...props
}: HTMLAttributes<HTMLTableSectionElement>) {
  return <thead {...props} className={`ui-table-header ${className}`} />;
}
export function TableRow({
  className = "",
  ...props
}: HTMLAttributes<HTMLTableRowElement>) {
  return <tr {...props} className={`ui-table-row ${className}`} />;
}
export function TableHead({
  className = "",
  scope = "col",
  ...props
}: ThHTMLAttributes<HTMLTableCellElement>) {
  return (
    <th {...props} scope={scope} className={`ui-table-head ${className}`} />
  );
}
export function TableCell({
  className = "",
  ...props
}: TdHTMLAttributes<HTMLTableCellElement>) {
  return <td {...props} className={`ui-table-cell ${className}`} />;
}
