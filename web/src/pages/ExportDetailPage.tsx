import { useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Copy } from 'lucide-react'
import { fetchExport } from '@/api/finetune'
import { PollingCard } from '@/components/PollingCard'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card'
import { fmtDate } from '@/lib/utils'

export function ExportDetailPage() {
  const { id } = useParams<{ id: string }>()

  const q = useQuery({
    queryKey: ['export', id],
    queryFn: () => fetchExport(id!),
    enabled: !!id,
    refetchInterval: (query) => {
      const s = query.state.data?.status
      if (s === 'complete' || s === 'error') return false
      return 3000
    },
  })

  const ex = q.data

  return (
    <div className="mx-auto max-w-2xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Fine-tune Export</h1>
        <p className="font-mono text-xs text-gray-400 mt-0.5">{id}</p>
      </div>

      <PollingCard
        status={ex?.status ?? 'pending'}
        label="Generating export…"
      >
        {ex && (
          <div className="grid grid-cols-2 gap-3 text-sm">
            <Meta label="Run ID" value={ex.run_id.slice(0, 8) + '…'} />
            <Meta label="Format" value={ex.format} />
            <Meta label="Row Count" value={ex.row_count != null ? String(ex.row_count) : '—'} />
            <Meta label="Created" value={fmtDate(ex.created_at)} />
            {ex.completed_at && (
              <Meta label="Completed" value={fmtDate(ex.completed_at)} />
            )}
          </div>
        )}
      </PollingCard>

      {ex?.file_path && (
        <Card>
          <CardHeader><CardTitle>Output File</CardTitle></CardHeader>
          <CardContent>
            <div className="flex items-center gap-2 rounded-md border border-gray-200 bg-gray-50 px-3 py-2 font-mono text-xs text-gray-700">
              <span className="flex-1 break-all">{ex.file_path}</span>
              <button
                onClick={() => navigator.clipboard.writeText(ex.file_path!)}
                className="text-gray-400 hover:text-gray-700 flex-shrink-0"
                title="Copy path"
              >
                <Copy className="h-3.5 w-3.5" />
              </button>
            </div>
            <p className="mt-2 text-xs text-gray-500">
              Copy this path to use with your fine-tuning pipeline.
            </p>
          </CardContent>
        </Card>
      )}

      {ex?.status === 'error' && (
        <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
          Export failed. Check the API logs for details.
        </div>
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
