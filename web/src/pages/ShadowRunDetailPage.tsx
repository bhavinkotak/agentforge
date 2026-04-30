import { useParams, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { fetchShadowRun } from '@/api/shadow'
import { PollingCard } from '@/components/PollingCard'
import { Card, CardHeader, CardTitle } from '@/components/ui/Card'
import { fmtDate, fmtScore } from '@/lib/utils'
import { cn } from '@/lib/utils'
import type { DimensionComparison } from '@/types'

export function ShadowRunDetailPage() {
  const { id } = useParams<{ id: string }>()

  const q = useQuery({
    queryKey: ['shadow-run', id],
    queryFn: () => fetchShadowRun(id!),
    enabled: !!id,
    refetchInterval: (query) => {
      const s = query.state.data?.status
      if (s === 'complete' || s === 'error') return false
      return 3000
    },
  })

  const run = q.data

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Shadow Run</h1>
        <p className="font-mono text-xs text-gray-400 mt-0.5">{id}</p>
      </div>

      <PollingCard
        status={run?.status ?? 'pending'}
        label="Comparing champion vs. candidate…"
      >
        {run && (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 text-sm">
            <Meta label="Champion" value={run.champion_agent_id.slice(0, 8) + '…'} />
            <Meta label="Candidate" value={run.candidate_agent_id.slice(0, 8) + '…'} />
            <Meta label="Traffic %" value={`${run.traffic_percent}%`} />
            <Meta label="Created" value={fmtDate(run.created_at)} />
          </div>
        )}
      </PollingCard>

      {/* Comparison results */}
      {run?.comparison && (
        <div className="space-y-4">
          {/* Aggregate */}
          <div className="grid grid-cols-3 gap-4">
            <ScoreCard
              label="Champion Score"
              value={fmtScore(run.comparison.champion_aggregate_score)}
            />
            <ScoreCard
              label="Candidate Score"
              value={fmtScore(run.comparison.candidate_aggregate_score)}
              highlight
            />
            <ScoreCard
              label="Delta"
              value={(run.comparison.aggregate_delta >= 0 ? '+' : '') + fmtScore(run.comparison.aggregate_delta)}
              positive={run.comparison.aggregate_delta >= 0}
            />
          </div>

          <div className="grid grid-cols-2 gap-3 text-sm">
            <Meta label="Total Requests" value={String(run.comparison.total_requests)} />
            <Meta label="Candidate Wins" value={`${run.comparison.candidate_wins} / 6 dimensions`} />
            <Meta label="Compared At" value={fmtDate(run.comparison.compared_at)} />
          </div>

          {/* Per-dimension table */}
          <Card>
            <CardHeader><CardTitle>Dimension Comparison</CardTitle></CardHeader>
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-gray-100 bg-gray-50">
                    <Th>Dimension</Th>
                    <Th>Champion</Th>
                    <Th>Candidate</Th>
                    <Th>Delta</Th>
                    <Th>Result</Th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {run.comparison.per_dimension.map((dim) => (
                    <DimRow key={dim.dimension} dim={dim} />
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      )}

      {/* Agent links */}
      {run && (
        <div className="flex gap-2 text-sm">
          <Link to={`/agents/${run.champion_agent_id}`} className="text-indigo-600 hover:underline">
            Champion →
          </Link>
          <Link to={`/agents/${run.candidate_agent_id}`} className="text-indigo-600 hover:underline">
            Candidate →
          </Link>
        </div>
      )}
    </div>
  )
}

function DimRow({ dim }: { dim: DimensionComparison }) {
  const outcomeColor =
    dim.outcome === 'Win'
      ? 'bg-green-50 text-green-700'
      : dim.outcome === 'Loss'
        ? 'bg-red-50 text-red-700'
        : 'bg-gray-50 text-gray-500'

  return (
    <tr className="hover:bg-gray-50 transition-colors">
      <td className="px-4 py-2 text-gray-700 capitalize">{dim.dimension.replace(/_/g, ' ')}</td>
      <td className="px-4 py-2 tabular-nums text-gray-600">{fmtScore(dim.champion_score)}</td>
      <td className="px-4 py-2 tabular-nums text-gray-600">{fmtScore(dim.candidate_score)}</td>
      <td className={cn('px-4 py-2 tabular-nums font-medium', dim.delta >= 0 ? 'text-green-600' : 'text-red-600')}>
        {dim.delta >= 0 ? '+' : ''}{fmtScore(dim.delta)}
      </td>
      <td className="px-4 py-2">
        <span className={cn('rounded-full px-2 py-0.5 text-xs font-medium', outcomeColor)}>
          {dim.outcome}
        </span>
      </td>
    </tr>
  )
}

function Th({ children }: { children: React.ReactNode }) {
  return (
    <th className="px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wide text-gray-500">
      {children}
    </th>
  )
}

function Meta({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs text-gray-500">{label}</p>
      <p className="mt-0.5 text-sm font-medium text-gray-900">{value}</p>
    </div>
  )
}

function ScoreCard({ label, value, highlight, positive }: {
  label: string; value: string; highlight?: boolean; positive?: boolean
}) {
  return (
    <div className={cn('rounded-lg border p-4', highlight ? 'border-indigo-200 bg-indigo-50' : positive != null ? (positive ? 'border-green-200 bg-green-50' : 'border-red-200 bg-red-50') : 'border-gray-200 bg-white')}>
      <p className="text-xs text-gray-500">{label}</p>
      <p className="mt-1 text-2xl font-semibold tabular-nums text-gray-900">{value}</p>
    </div>
  )
}
