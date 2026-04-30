import { useState, useEffect } from 'react'
import { useSearchParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { fetchDiff } from '@/api/diff'
import { DiffViewer } from '@/components/DiffViewer'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { Card, CardContent } from '@/components/ui/Card'

export function DiffPage() {
  const [searchParams, setSearchParams] = useSearchParams()
  const [v1, setV1] = useState(searchParams.get('v1') ?? '')
  const [v2, setV2] = useState(searchParams.get('v2') ?? '')
  const [submitted, setSubmitted] = useState(!!searchParams.get('v1') && !!searchParams.get('v2'))

  const diffQ = useQuery({
    queryKey: ['diff', v1, v2],
    queryFn: () => fetchDiff(v1, v2),
    enabled: submitted && !!v1 && !!v2,
    retry: false,
  })

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setSearchParams({ v1, v2 })
    setSubmitted(true)
  }

  useEffect(() => {
    const p1 = searchParams.get('v1')
    const p2 = searchParams.get('v2')
    if (p1) setV1(p1)
    if (p2) setV2(p2)
    if (p1 && p2) setSubmitted(true)
  }, [searchParams])

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Version Diff</h1>
        <p className="mt-0.5 text-sm text-gray-500">
          Compare two agent versions side by side.
        </p>
      </div>

      <form onSubmit={handleSubmit}>
        <Card>
          <CardContent className="flex flex-col gap-3 sm:flex-row sm:items-end">
            <div className="flex-1">
              <Input
                label="Agent v1 (UUID)"
                placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                value={v1}
                onChange={e => setV1(e.target.value)}
                required
              />
            </div>
            <div className="flex-1">
              <Input
                label="Agent v2 (UUID)"
                placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                value={v2}
                onChange={e => setV2(e.target.value)}
                required
              />
            </div>
            <Button type="submit">Compare →</Button>
          </CardContent>
        </Card>
      </form>

      {diffQ.isLoading && (
        <p className="text-sm text-gray-500">Loading diff…</p>
      )}

      {diffQ.error && (
        <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
          {diffQ.error instanceof Error ? diffQ.error.message : 'Failed to load diff'}
        </div>
      )}

      {diffQ.data && <DiffViewer diff={diffQ.data} />}
    </div>
  )
}
