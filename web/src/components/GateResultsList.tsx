import { CheckCircle, XCircle, MinusCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import type { GateResult } from '@/types'

interface Props {
  gates: GateResult[]
  approved: boolean
  changelog?: string
}

export function GateResultsList({ gates, approved, changelog }: Props) {
  return (
    <div className="space-y-4">
      <div
        className={cn(
          'flex items-center gap-3 rounded-lg border px-4 py-3 text-sm font-medium',
          approved
            ? 'border-green-200 bg-green-50 text-green-800'
            : 'border-red-200 bg-red-50 text-red-800',
        )}
      >
        {approved ? (
          <CheckCircle className="h-5 w-5 text-green-600" />
        ) : (
          <XCircle className="h-5 w-5 text-red-600" />
        )}
        Promotion {approved ? 'approved' : 'denied'}
      </div>

      <div className="rounded-lg border border-gray-200 bg-white divide-y divide-gray-100">
        {gates.map((gate) => (
          <GateRow key={gate.gate} gate={gate} />
        ))}
      </div>

      {changelog && (
        <div className="rounded-lg border border-gray-200 bg-white p-4">
          <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-gray-400">
            Changelog
          </p>
          <p className="whitespace-pre-wrap text-sm text-gray-700">{changelog}</p>
        </div>
      )}
    </div>
  )
}

function GateRow({ gate }: { gate: GateResult }) {
  const icon =
    gate.status === 'pass' ? (
      <CheckCircle className="h-4 w-4 text-green-500" />
    ) : gate.status === 'waived' ? (
      <MinusCircle className="h-4 w-4 text-gray-400" />
    ) : (
      <XCircle className="h-4 w-4 text-red-500" />
    )

  return (
    <div className="flex items-start gap-3 px-4 py-3">
      <span className="mt-0.5">{icon}</span>
      <div>
        <p className="text-sm font-medium text-gray-900">{gate.gate}</p>
        <p className="text-xs text-gray-500">{gate.message}</p>
      </div>
    </div>
  )
}
