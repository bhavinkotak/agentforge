import { apiFetch } from './client'
import type { BenchmarkRunResponse } from '@/types'

export interface StartBenchmarkRequest {
  agent_id: string
  suite: 'gaia' | 'agentbench' | 'webarena'
}

export const startBenchmark = (req: StartBenchmarkRequest) =>
  apiFetch<BenchmarkRunResponse>('/benchmarks', {
    method: 'POST',
    body: JSON.stringify(req),
  })

export const fetchBenchmark = (id: string) =>
  apiFetch<BenchmarkRunResponse>(`/benchmarks/${id}`)
