import { apiFetch } from './client'
import type { FineTuneExportResponse } from '@/types'

export interface StartExportRequest {
  run_id: string
  format?: 'openai' | 'anthropic' | 'huggingface'
}

export const startExport = (req: StartExportRequest) =>
  apiFetch<FineTuneExportResponse>('/exports/finetune', {
    method: 'POST',
    body: JSON.stringify(req),
  })

export const fetchExport = (id: string) =>
  apiFetch<FineTuneExportResponse>(`/exports/finetune/${id}`)
