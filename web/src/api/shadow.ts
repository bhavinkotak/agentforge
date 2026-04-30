import { apiFetch } from './client'
import type { ShadowRunResponse } from '@/types'

export interface StartShadowRunRequest {
  champion_agent_id: string
  candidate_agent_id: string
  traffic_percent?: number
}

export const startShadowRun = (req: StartShadowRunRequest) =>
  apiFetch<ShadowRunResponse>('/shadow-runs', {
    method: 'POST',
    body: JSON.stringify(req),
  })

export const fetchShadowRun = (id: string) =>
  apiFetch<ShadowRunResponse>(`/shadow-runs/${id}`)
