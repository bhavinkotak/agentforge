import { apiFetch } from './client'
import type { DiffResponse } from '@/types'

export const fetchDiff = (v1: string, v2: string) =>
  apiFetch<DiffResponse>(`/diff?v1=${v1}&v2=${v2}`)
