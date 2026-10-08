import { SVGProps } from "react";

/** خصائص مشتركة لكل الأيقونات — حجم وسماكة خط موحَّدان عبر التطبيق كله */
type IconProps = Omit<SVGProps<SVGSVGElement>, "viewBox" | "fill"> & { size?: number };

function base(size: number) {
  return {
    width: size,
    height: size,
    viewBox: "0 0 20 20",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.6,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
  };
}

export function IconSettings({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <circle cx="10" cy="10" r="2.4" />
      <path d="M10 2.5v2M10 15.5v2M17.5 10h-2M4.5 10h-2M15.1 4.9l-1.4 1.4M6.3 13.7l-1.4 1.4M15.1 15.1l-1.4-1.4M6.3 6.3L4.9 4.9" />
    </svg>
  );
}

export function IconRecycle({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M8.5 3.2 6.2 7.1M11.8 3.2l2.3 3.9M3.6 12.4l2.2 3.9h3.9M16.4 12.4l-2.2 3.9h-3.9" />
      <path d="M5.2 10.6 3.6 12.4l1.9 1.5M14.8 10.6l1.6 1.8-1.9 1.5" />
    </svg>
  );
}

export function IconLock({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <rect x="4.5" y="9" width="11" height="8" rx="1.5" />
      <path d="M6.5 9V6.5a3.5 3.5 0 0 1 7 0V9" />
    </svg>
  );
}

export function IconScale({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M10 3v14M6 17h8M10 3l-4.5 3M10 3l4.5 3" />
      <path d="M2.5 6l3-1 3 1-3 5-3-5ZM11.5 6l3-1 3 1-3 5-3-5Z" />
    </svg>
  );
}

export function IconBox({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M10 2.5 17 6.3v7.4L10 17.5 3 13.7V6.3L10 2.5Z" />
      <path d="M3 6.3 10 10l7-3.7M10 10v7.5" />
    </svg>
  );
}

export function IconNote({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M5 3h7l3 3v11a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1Z" />
      <path d="M12 3v3h3M7 10.5h6M7 13.5h4" />
    </svg>
  );
}

export function IconQr({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <rect x="3" y="3" width="5.5" height="5.5" rx="0.8" />
      <rect x="11.5" y="3" width="5.5" height="5.5" rx="0.8" />
      <rect x="3" y="11.5" width="5.5" height="5.5" rx="0.8" />
      <path d="M12.3 12.3h1.8v1.8h-1.8zM15.6 12.3h1.4M12.3 15.6h1.4M15.9 15.9h1.1" />
    </svg>
  );
}

export function IconRoute({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <circle cx="5" cy="5" r="2" />
      <circle cx="15" cy="15" r="2" />
      <path d="M5 7v2a3 3 0 0 0 3 3h4a3 3 0 0 1 3 3" strokeDasharray="2.3 2.3" />
    </svg>
  );
}

export function IconLogout({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M8 17H4.5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1H8M13 14l4-4-4-4M17 10H7.5" />
    </svg>
  );
}

export function IconClose({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M5 5l10 10M15 5 5 15" />
    </svg>
  );
}

export function IconSearch({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <circle cx="8.8" cy="8.8" r="5.3" />
      <path d="m16.5 16.5-3.6-3.6" />
    </svg>
  );
}

export function IconPlus({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M10 4v12M4 10h12" />
    </svg>
  );
}

export function IconTrash({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M4 5.5h12M8 5.5V4a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v1.5M6 5.5 6.8 16a1 1 0 0 0 1 .9h4.4a1 1 0 0 0 1-.9l.8-10.5" />
    </svg>
  );
}

export function IconDashboard({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <rect x="2.5" y="2.5" width="7" height="6" rx="1.2" />
      <rect x="10.5" y="2.5" width="7" height="3.5" rx="1.2" />
      <rect x="10.5" y="7.5" width="7" height="9.5" rx="1.2" />
      <rect x="2.5" y="10" width="7" height="7" rx="1.2" />
    </svg>
  );
}

export function IconArchive({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <rect x="2.5" y="3" width="15" height="4" rx="1" />
      <path d="M3.5 7v8a1.5 1.5 0 0 0 1.5 1.5h10A1.5 1.5 0 0 0 16.5 15V7" />
      <path d="M8 10.5h4" />
    </svg>
  );
}

export function IconInbox({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M3 10.5 5 4h10l2 6.5" />
      <path d="M3 10.5v4.5a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-4.5h-4.3a2.2 2.2 0 0 1-4.4 0H3Z" />
    </svg>
  );
}

export function IconUsers({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <circle cx="7.3" cy="6.5" r="2.3" />
      <path d="M2.5 16c.4-2.8 2.3-4.5 4.8-4.5s4.4 1.7 4.8 4.5" />
      <circle cx="13.8" cy="7" r="1.8" />
      <path d="M12.8 11.7c2 .2 3.4 1.8 3.7 4.3" />
    </svg>
  );
}

export function IconUpload({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M10 13V3.5M6 7l4-4 4 4M4 16.5h12" />
    </svg>
  );
}

export function IconDownload({ size = 16, ...p }: IconProps) {
  return (
    <svg {...base(size)} {...p}>
      <path d="M10 3v9.5M6 9l4 4 4-4M4 16.5h12" />
    </svg>
  );
}

export function IconSpinner({ size = 14, ...p }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      className="animate-spin"
      {...p}
    >
      <circle cx="10" cy="10" r="7.5" stroke="currentColor" strokeOpacity="0.25" strokeWidth="2.2" />
      <path d="M17.5 10a7.5 7.5 0 0 0-7.5-7.5" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
    </svg>
  );
}
