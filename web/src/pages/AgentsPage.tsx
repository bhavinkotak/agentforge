import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Plus, Search } from 'lucide-react'
import { fetchAgents } from '@/api/agents'
import { RunStatusBadge } from '@/components/RunStatusBadge'
import { Button } from '@/components/ui/Button'
import { Input } from '@/components/ui/Input'
import { Card } from '@/components/ui/Card'
import { fmtDate, truncate } from '@/lib/utils'

export function AgentsPage() {
  const [offset, setOffset] = useState(0)
  const [search, setSearch] = useState('')
  const LIMIT = 20

  const { data, isLoading } = useQuery({
    queryKey: ['agents', offset],
    queryFn: () => fetchAgents(LIMIT, offset),
  })

  const filtered = data?.filter(
    (a) =>
      !search ||
      a.name.toLowerCase().includes(search.toLowerCase()) ||
      a.version.includes(search),
  ) ?? []

  return (
    <div className="mx-auto max-w-5xl space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-gray-900">Agents</h1>
          <p className="mt-0.5 text-sm text-gray-500">
            All agent versions registered in the system.
          </p>
        </div>
        <Link to="/agents/new">
          <Button>
            <Plus className="h-4 w-4" />
            New Agent
          </Button>
        </Link>
      </div>

      <div className="max-w-xs">
        <Input
          placeholder="Filter by name or version…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="pl-8"
        />
        <Search className="-mt-7 ml-2.5 h-4 w-4 text-gray-400 relative pointer-events-none" />
      </div>

      <Card>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-100 bg-gray-50">
                <Th>Name</Th>
                <Th>Version</Th>
                <Th>Format</Th>
                <Th>SHA</Th>
                <Th>Status</Th>
                <Th>Created</Th>
                <Th />
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {isLoading && (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-center text-gray-500">
                    Loading…
                  </td>
                </tr>
              )}
              {!isLoading && filtered.length === 0 && (
                <tr>
                  <td colSpan={7} className="px-4 py-6 text-center text-gray-500">
                    No agents found.
                  </td>
                </tr>
              )}
              {filtered.map((agent) => (
                <tr key={agent.id} className="hover:bg-gray-50 transition-colors">
                  <td className="px-4 py-3 font-medium text-gray-900">{agent.name}</td>
                  <td className="px-4 py-3 text-gray-600">v{agent.version}</td>
                  <td className="px-4 py-3 text-gray-500">{agent.format}</td>
                  <td className="px-4 py-3 font-mono text-gray-400">
                    {truncate(agent.sha, 10)}
                  </td>
                  <td className="px-4 py-3">
                    {agent.is_champion && <RunStatusBadge status="champion" />}
                  </td>
                  <td className="px-4 py-3 text-gray-400">{fmtDate(agent.created_at)}</td>
                  <td className="px-4 py-3">
                    <Link
                      to={`/agents/${agent.id}`}
                      className="text-indigo-600 hover:underline"
                    >
                      View →
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        {(data?.length === LIMIT || offset > 0) && (
          <div className="flex items-center justify-between border-t border-gray-100 px-4 py-3">
            <Button
              variant="outline"
              size="sm"
              disabled={offset === 0}
              onClick={() => setOffset(Math.max(0, offset - LIMIT))}
            >
              ← Previous
            </Button>
            <span className="text-xs text-gray-500">
              Showing {offset + 1}–{offset + (data?.length ?? 0)}
            </span>
            <Button
              variant="outline"
              size="sm"
              disabled={(data?.length ?? 0) < LIMIT}
              onClick={() => setOffset(offset + LIMIT)}
            >
              Next →
            </Button>
          </div>
        )}
      </Card>
    </div>
  )
}

function Th({ children }: { children?: React.ReactNode }) {
  return (
    <th className="px-4 py-2.5 text-left text-xs font-semibold uppercase tracking-wide text-gray-500">
      {children}
    </th>
  )
}
