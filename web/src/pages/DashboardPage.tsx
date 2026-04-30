import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { Bot, Activity, ArrowRight } from 'lucide-react'
import { fetchAgents } from '@/api/agents'
import { RunStatusBadge } from '@/components/RunStatusBadge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card'
import { Button } from '@/components/ui/Button'
import { fmtDate } from '@/lib/utils'
import type { RecentItem } from '@/hooks/useRecentItems'

const RECENT_KEY = 'agentforge:recent'

function readRecent(): RecentItem[] {
  try {
    return JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]') as RecentItem[]
  } catch {
    return []
  }
}

const typeHref: Record<string, string> = {
  run: '/runs',
  shadow: '/shadow-runs',
  export: '/exports/finetune',
  benchmark: '/benchmarks',
}

const typeLabel: Record<string, string> = {
  run: 'Eval Run',
  shadow: 'Shadow Run',
  export: 'Export',
  benchmark: 'Benchmark',
}

export function DashboardPage() {
  const [recent, setRecent] = useState<RecentItem[]>([])
  useEffect(() => {
    setRecent(readRecent().slice(0, 8))
  }, [])

  const agentsQ = useQuery({
    queryKey: ['agents', 1],
    queryFn: () => fetchAgents(5, 0),
  })

  return (
    <div className="mx-auto max-w-4xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Dashboard</h1>
        <p className="mt-1 text-sm text-gray-500">
          Welcome to AgentForge — one file in, a better agent out.
        </p>
      </div>

      {/* Quick actions */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
        <QuickCard
          to="/agents/new"
          icon={<Bot className="h-5 w-5 text-indigo-600" />}
          title="Create Agent"
          desc="Upload an agent YAML / JSON file"
        />
        <QuickCard
          to="/agents"
          icon={<Activity className="h-5 w-5 text-green-600" />}
          title="View Agents"
          desc="Browse and manage agent versions"
        />
        <QuickCard
          to="/diff"
          icon={<ArrowRight className="h-5 w-5 text-orange-500" />}
          title="Compare Versions"
          desc="Diff two agent versions side by side"
        />
      </div>

      {/* Recent agents */}
      <Card>
        <CardHeader className="flex items-center justify-between">
          <CardTitle>Recent Agents</CardTitle>
          <Link to="/agents">
            <Button variant="ghost" size="sm">
              View all →
            </Button>
          </Link>
        </CardHeader>
        <CardContent className="p-0">
          {agentsQ.isLoading && (
            <p className="px-5 py-4 text-sm text-gray-500">Loading…</p>
          )}
          {agentsQ.data && agentsQ.data.length === 0 && (
            <p className="px-5 py-4 text-sm text-gray-500">
              No agents yet.{' '}
              <Link to="/agents/new" className="text-indigo-600 hover:underline">
                Create one →
              </Link>
            </p>
          )}
          {agentsQ.data?.map((agent) => (
            <Link
              key={agent.id}
              to={`/agents/${agent.id}`}
              className="flex items-center justify-between border-b border-gray-100 px-5 py-3 last:border-0 hover:bg-gray-50 transition-colors"
            >
              <div>
                <p className="text-sm font-medium text-gray-900">{agent.name}</p>
                <p className="text-xs text-gray-400">
                  v{agent.version} · {agent.format}
                </p>
              </div>
              {agent.is_champion && (
                <RunStatusBadge status="champion" />
              )}
            </Link>
          ))}
        </CardContent>
      </Card>

      {/* Recent activity */}
      {recent.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Recent Activity</CardTitle>
          </CardHeader>
          <CardContent className="p-0">
            {recent.map((item) => (
              <Link
                key={item.id}
                to={`${typeHref[item.type]}/${item.id}`}
                className="flex items-center justify-between border-b border-gray-100 px-5 py-3 last:border-0 hover:bg-gray-50 transition-colors"
              >
                <div>
                  <p className="text-xs font-medium text-indigo-600 uppercase tracking-wide">
                    {typeLabel[item.type]}
                  </p>
                  <p className="text-sm text-gray-700">{item.label}</p>
                </div>
                <span className="text-xs text-gray-400">
                  {fmtDate(new Date(item.timestamp).toISOString())}
                </span>
              </Link>
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  )
}

function QuickCard({
  to,
  icon,
  title,
  desc,
}: {
  to: string
  icon: React.ReactNode
  title: string
  desc: string
}) {
  return (
    <Link
      to={to}
      className="rounded-lg border border-gray-200 bg-white p-4 shadow-sm hover:border-indigo-300 hover:shadow transition-all"
    >
      <div className="mb-3">{icon}</div>
      <p className="text-sm font-medium text-gray-900">{title}</p>
      <p className="mt-0.5 text-xs text-gray-500">{desc}</p>
    </Link>
  )
}
