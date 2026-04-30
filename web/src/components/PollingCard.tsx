import { Loader2 } from 'lucide-react'
import { RunStatusBadge } from '@/components/RunStatusBadge'

interface Props {
  status: string
  label: string
  children: React.ReactNode
  isLoading?: boolean
}

/** Wrapper that shows a spinner while a job is running, content when done */
export function PollingCard({ status, label, children, isLoading }: Props) {
  const isTerminal = status === 'complete' || status === 'error'

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-3">
        <RunStatusBadge status={status} />
        {!isTerminal && (
          <span className="flex items-center gap-1.5 text-sm text-gray-500">
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
            {label}
          </span>
        )}
        {isLoading && isTerminal && (
          <Loader2 className="h-3.5 w-3.5 animate-spin text-gray-400" />
        )}
      </div>
      {children}
    </div>
  )
}
