/// <reference types="vite/client" />
import type { ApiErrorBody } from '@/types'

const BASE = import.meta.env.VITE_API_BASE ?? '/api'

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

export async function apiFetch<T>(
  path: string,
  init?: RequestInit,
): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  })

  if (!res.ok) {
    let code = 'UNKNOWN'
    let message = `HTTP ${res.status}`
    try {
      const body = (await res.json()) as ApiErrorBody
      code = body.error.code
      message = body.error.message
    } catch {
      // ignore parse failure
    }
    throw new ApiError(res.status, code, message)
  }

  // 202 / 204 may have no body
  if (res.status === 204) return undefined as T
  return res.json() as Promise<T>
}
