import { clsx, type ClassValue } from 'clsx'

export function cn(...inputs: ClassValue[]) {
  return clsx(inputs)
}

export function fmtDate(iso?: string | null) {
  if (!iso) return '—'
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(iso))
}

export function fmtPct(n?: number | null) {
  if (n == null) return '—'
  return `${(n * 100).toFixed(1)}%`
}

export function fmtScore(n?: number | null) {
  if (n == null) return '—'
  return n.toFixed(3)
}

export function truncate(s: string, len = 12) {
  return s.length <= len ? s : s.slice(0, len) + '…'
}
