import { cn } from '@/lib/utils'

type Variant = 'pending' | 'running' | 'complete' | 'error' | 'champion' | 'default'

const styles: Record<Variant, string> = {
  pending:  'bg-yellow-50 text-yellow-700 ring-yellow-200',
  running:  'bg-blue-50 text-blue-700 ring-blue-200',
  complete: 'bg-green-50 text-green-700 ring-green-200',
  error:    'bg-red-50 text-red-700 ring-red-200',
  champion: 'bg-indigo-50 text-indigo-700 ring-indigo-200',
  default:  'bg-gray-100 text-gray-600 ring-gray-200',
}

const dots: Record<Variant, string> = {
  pending:  'bg-yellow-400',
  running:  'bg-blue-500 animate-pulse',
  complete: 'bg-green-500',
  error:    'bg-red-500',
  champion: 'bg-indigo-500',
  default:  'bg-gray-400',
}

interface Props {
  status: string
  className?: string
}

export function RunStatusBadge({ status, className }: Props) {
  const variant = (status as Variant) in styles ? (status as Variant) : 'default'
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium ring-1 ring-inset',
        styles[variant],
        className,
      )}
    >
      <span className={cn('h-1.5 w-1.5 rounded-full', dots[variant])} />
      {status}
    </span>
  )
}
