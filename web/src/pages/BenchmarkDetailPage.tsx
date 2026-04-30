import { useParams, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { fetchBenchmark } from '@/api/benchmarks'
import { PollingCard } from '@/components/PollingCard'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card'
import { fmtDate, fmtPct } from '@/lib/utils'
import { cn } from '@/lib/utils'

export function BenchmarkDetailPage() {
  const { id } = useParams<{ id: string }>()

  const q = useQuery({
    queryKey: ['benchmark', id],
    queryFn: () => fetchBenchmark(id!),
    enabled: !!id,
    refetchInterval: (query) => {
      const s = query.state.data?.status
      if (s === 'complete' || s === 'error') return false
      return 3000
    },
  })

  const run = q.data

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Benchmark Run</h1>
        <p className="font-mono text-xs text-gray-400 mt-0.5">{id}</p>
      </div>

      <PollingCard
        status={run?.status ?? 'pending'}
        label="Running benchmark tasks…"
      >
        {run && (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 text-sm">
            <Meta label="Suite" value={run.suite.toUpperCase()} />
            <Meta label="Agent" value={run.agent_id.slice(0, 8) + '…'} />
            <Meta label="Started" value={fmtDate(run.started_at)} />
          </div>
        )}
      </PollingCard>

      {/* Results — shown when complete */}
      {run?.status === 'complete' && (
        <div className="space-y-4">
          {/* Metrics grid */}
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            <StatCard label="Accuracy" value={fmtPct(run.accuracy)} highlight={run.accuracy >= 0.5} />
            <StatCard label="Correct" value={`${run.correct} / ${run.total_tasks}`} />
            {run.percentile_rank != null && (
              <StatCard label="Percentile" value={`${run.percentile_rank.toFixed(0)}th`} />
            )}
            {run.completed_at && (
              <StatCard label="Completed" value={fmtDate(run.completed_at)} />
            )}
          </div>

          {/* Percentile gauge */}
          {run.percentile_rank != null && (
            <Card>
              <CardHeader><CardTitle>vs. Published Baselines</CardTitle></CardHeader>
              <CardContent>
                <div className="space-y-2">
                  <div className="flex items-center justify-between text-sm">
                    <span className="text-gray-600">Percentile rank</span>
                    <span className="font-semibold text-gray-900">
                      {run.percentile_rank.toFixed(0)}th percentile
                    </span>
                  </div>
                  <div className="h-3 overflow-hidden rounded-full bg-gray-100">
                    <div
                      className={cn(
                        'h-3 rounded-full',
                        run.percentile_rank >= 75
                          ? 'bg-green-500'
                          : run.percentile_rank >= 50
                            ? 'bg-yellow-500'
                            : 'bg-red-500',
                      )}
                      style={{ width: `${run.percentile_rank}%` }}
                    />
                  </div>
                  <p className="text-xs text-gray-400">
                    Compared against published results for the {run.suite} suite.
                  </p>
                </div>
              </CardContent>
            </Card>
          )}
        </div>
      )}

      {run && (
        <Link to={`/agents/${run.agent_id}`} className="text-sm text-indigo-600 hover:underline">
          ← Back to agent
        </Link>
      )}
    </div>
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

function StatCard({ label, value, highlight }: { label: string; value: string; highlight?: boolean }) {
  return (
    <div className={cn('rounded-lg border p-4', highlight ? 'border-green-200 bg-green-50' : 'border-gray-200 bg-white')}>
      <p className="text-xs text-gray-500">{label}</p>
      <p className={cn('mt-1 text-xl font-semibold tabular-nums', highlight ? 'text-green-700' : 'text-gray-900')}>
        {value}
      </p>
    </div>
  )
}
