import { useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { useQuery, useMutation } from '@tanstack/react-query'
import { Play, GitCompare, Cpu, FileDown, Copy } from 'lucide-react'
import { fetchAgent } from '@/api/agents'
import { startRun } from '@/api/runs'
import { startShadowRun } from '@/api/shadow'
import { startBenchmark } from '@/api/benchmarks'
import { ApiError } from '@/api/client'
import { RunStatusBadge } from '@/components/RunStatusBadge'
import { Button } from '@/components/ui/Button'
import { Input, Select } from '@/components/ui/Input'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card'
import { useRecentItems } from '@/hooks/useRecentItems'
import { fmtDate, truncate } from '@/lib/utils'

type Panel = null | 'run' | 'shadow' | 'benchmark'

export function AgentDetailPage() {
  const { id } = useParams<{ id: string }>()
  const navigate = useNavigate()
  const { push } = useRecentItems()
  const [openPanel, setOpenPanel] = useState<Panel>(null)
  const [err, setErr] = useState<string | null>(null)

  const agentQ = useQuery({
    queryKey: ['agent', id],
    queryFn: () => fetchAgent(id!),
    enabled: !!id,
  })

  // Run form state
  const [scenarios, setScenarios] = useState('100')
  const [concurrency, setConcurrency] = useState('10')
  const [seed, setSeed] = useState('42')

  // Shadow form state
  const [candidateId, setCandidateId] = useState('')
  const [trafficPct, setTrafficPct] = useState('10')

  // Benchmark form state
  const [suite, setSuite] = useState<'gaia' | 'agentbench' | 'webarena'>('gaia')

  const runMut = useMutation({ mutationFn: startRun })
  const shadowMut = useMutation({ mutationFn: startShadowRun })
  const benchMut = useMutation({ mutationFn: startBenchmark })

  async function handleStartRun(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      const run = await runMut.mutateAsync({
        agent_id: id!,
        scenario_count: parseInt(scenarios),
        concurrency: parseInt(concurrency),
        seed: parseInt(seed),
      })
      push({ id: run.id, type: 'run', label: `Run for ${agentQ.data?.name}`, timestamp: Date.now() })
      navigate(`/runs/${run.id}`)
    } catch (e) {
      setErr(e instanceof ApiError ? e.message : 'Failed to start run')
    }
  }

  async function handleStartShadow(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      const sr = await shadowMut.mutateAsync({
        champion_agent_id: id!,
        candidate_agent_id: candidateId,
        traffic_percent: parseInt(trafficPct),
      })
      push({ id: sr.id, type: 'shadow', label: `Shadow: ${agentQ.data?.name}`, timestamp: Date.now() })
      navigate(`/shadow-runs/${sr.id}`)
    } catch (e) {
      setErr(e instanceof ApiError ? e.message : 'Failed to start shadow run')
    }
  }

  async function handleStartBenchmark(e: React.FormEvent) {
    e.preventDefault()
    setErr(null)
    try {
      const br = await benchMut.mutateAsync({ agent_id: id!, suite })
      push({ id: br.id, type: 'benchmark', label: `${suite} · ${agentQ.data?.name}`, timestamp: Date.now() })
      navigate(`/benchmarks/${br.id}`)
    } catch (e) {
      setErr(e instanceof ApiError ? e.message : 'Failed to start benchmark')
    }
  }

  if (agentQ.isLoading) return <p className="text-sm text-gray-500">Loading…</p>
  if (!agentQ.data) return <p className="text-sm text-red-600">Agent not found.</p>

  const agent = agentQ.data

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h1 className="text-xl font-semibold text-gray-900">{agent.name}</h1>
            {agent.is_champion && <RunStatusBadge status="champion" />}
          </div>
          <p className="mt-0.5 text-sm text-gray-500">
            v{agent.version} · {agent.format} · {fmtDate(agent.created_at)}
          </p>
        </div>
      </div>

      {/* Meta card */}
      <Card>
        <CardContent className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          <Meta label="Format" value={agent.format} />
          <Meta label="Version" value={`v${agent.version}`} />
          <Meta label="Promoted" value={agent.promoted ? 'Yes' : 'No'} />
          <Meta label="SHA" value={truncate(agent.sha, 12)} mono />
        </CardContent>
      </Card>

      {/* SHA copy */}
      <div className="flex items-center gap-2 rounded-md border border-gray-200 bg-gray-50 px-3 py-2 font-mono text-xs text-gray-600">
        <span className="flex-1">{agent.sha}</span>
        <button
          onClick={() => navigator.clipboard.writeText(agent.sha)}
          className="text-gray-400 hover:text-gray-700"
          title="Copy SHA"
        >
          <Copy className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Action buttons */}
      <div className="flex flex-wrap gap-2">
        <Button
          variant={openPanel === 'run' ? 'primary' : 'outline'}
          onClick={() => setOpenPanel(openPanel === 'run' ? null : 'run')}
        >
          <Play className="h-4 w-4" />
          Start Eval Run
        </Button>
        <Button
          variant={openPanel === 'shadow' ? 'primary' : 'outline'}
          onClick={() => setOpenPanel(openPanel === 'shadow' ? null : 'shadow')}
        >
          <GitCompare className="h-4 w-4" />
          Shadow Run
        </Button>
        <Button
          variant={openPanel === 'benchmark' ? 'primary' : 'outline'}
          onClick={() => setOpenPanel(openPanel === 'benchmark' ? null : 'benchmark')}
        >
          <Cpu className="h-4 w-4" />
          Benchmark
        </Button>
        <Button
          variant="outline"
          onClick={() => navigate(`/diff?v1=${agent.id}`)}
        >
          <FileDown className="h-4 w-4" />
          Diff
        </Button>
      </div>

      {err && (
        <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
          {err}
        </div>
      )}

      {/* Inline panels */}
      {openPanel === 'run' && (
        <Card>
          <CardHeader><CardTitle>Start Eval Run</CardTitle></CardHeader>
          <CardContent>
            <form onSubmit={handleStartRun} className="space-y-3">
              <div className="grid grid-cols-3 gap-3">
                <Input label="Scenarios" type="number" value={scenarios} onChange={e => setScenarios(e.target.value)} min={1} max={2000} />
                <Input label="Concurrency" type="number" value={concurrency} onChange={e => setConcurrency(e.target.value)} min={1} max={50} />
                <Input label="Seed" type="number" value={seed} onChange={e => setSeed(e.target.value)} />
              </div>
              <Button type="submit" loading={runMut.isPending}>Start →</Button>
            </form>
          </CardContent>
        </Card>
      )}

      {openPanel === 'shadow' && (
        <Card>
          <CardHeader><CardTitle>Shadow Run</CardTitle></CardHeader>
          <CardContent>
            <form onSubmit={handleStartShadow} className="space-y-3">
              <p className="text-xs text-gray-500">
                This agent is the <strong>champion</strong>. Provide the candidate agent ID below.
              </p>
              <Input
                label="Candidate Agent ID"
                placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
                value={candidateId}
                onChange={e => setCandidateId(e.target.value)}
                required
              />
              <Input label="Traffic % to Candidate" type="number" value={trafficPct} onChange={e => setTrafficPct(e.target.value)} min={1} max={100} />
              <Button type="submit" loading={shadowMut.isPending}>Start Shadow →</Button>
            </form>
          </CardContent>
        </Card>
      )}

      {openPanel === 'benchmark' && (
        <Card>
          <CardHeader><CardTitle>Benchmark</CardTitle></CardHeader>
          <CardContent>
            <form onSubmit={handleStartBenchmark} className="space-y-3">
              <Select
                label="Suite"
                value={suite}
                onChange={e => setSuite(e.target.value as typeof suite)}
                options={[
                  { value: 'gaia', label: 'GAIA' },
                  { value: 'agentbench', label: 'AgentBench' },
                  { value: 'webarena', label: 'WebArena' },
                ]}
              />
              <Button type="submit" loading={benchMut.isPending}>Start Benchmark →</Button>
            </form>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

function Meta({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <p className="text-xs text-gray-500">{label}</p>
      <p className={`mt-0.5 text-sm font-medium text-gray-900 ${mono ? 'font-mono' : ''}`}>{value}</p>
    </div>
  )
}
