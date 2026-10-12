import {
  Mail,
  Download,
  Printer,
  Copy,
  ChevronsLeft,
  ChevronsRight,
  Pencil,
  Ellipsis,
  Languages,
  LogOut,
  LayoutDashboard,
  Users,
  ShoppingBag,
  Package,
  Wallet,
  Wrench,
  Menu,
  CalendarDays,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Clock,
  Info,
  LoaderCircle,
  TriangleAlert,
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  Check,
  Eye,
  Plus,
  ReceiptText,
  Search,
  SlidersHorizontal,
  Trash2,
  X,
} from "lucide-react";

// Explicit imports keep unused icons out of the application bundle.
const icons = {
  mail: Mail,
  download: Download,
  print: Printer,
  copy: Copy,
  firstPage: ChevronsLeft,
  lastPage: ChevronsRight,
  edit: Pencil,
  more: Ellipsis,
  language: Languages,
  logout: LogOut,
  overview: LayoutDashboard,
  contacts: Users,
  expenses: ShoppingBag,
  catalog: Package,
  wallet: Wallet,
  tools: Wrench,
  menu: Menu,
  calendar: CalendarDays,
  chevronDown: ChevronDown,
  chevronLeft: ChevronLeft,
  chevronRight: ChevronRight,
  clock: Clock,
  info: Info,
  loading: LoaderCircle,
  warning: TriangleAlert,
  invoice: ReceiptText,
  plus: Plus,
  search: Search,
  arrow: ArrowRight,
  back: ArrowLeft,
  external: ArrowUpRight,
  check: Check,
  settings: SlidersHorizontal,
  trash: Trash2,
  eye: Eye,
  close: X,
} as const;
export function Icon({
  name,
  className = "h-4 w-4",
}: {
  name: keyof typeof icons;
  className?: string;
}) {
  const Component = icons[name];
  return (
    <Component
      aria-hidden="true"
      focusable="false"
      strokeWidth={1.75}
      className={`shrink-0 ${className}`}
    />
  );
}
