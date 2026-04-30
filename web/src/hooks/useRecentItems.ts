import { useCallback } from 'react'

export type ItemType = 'run' | 'shadow' | 'export' | 'benchmark'

export interface RecentItem {
  id: string
  type: ItemType
  label: string
  timestamp: number
}

const KEY = 'agentforge:recent'
const MAX = 20

function readItems(): RecentItem[] {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '[]') as RecentItem[]
  } catch {
    return []
  }
}

function writeItems(items: RecentItem[]) {
  localStorage.setItem(KEY, JSON.stringify(items))
}

export function useRecentItems() {
  const push = useCallback((item: RecentItem) => {
    const existing = readItems().filter((i) => i.id !== item.id)
    writeItems([item, ...existing].slice(0, MAX))
  }, [])

  const getAll = useCallback(() => readItems(), [])

  const getByType = useCallback(
    (type: ItemType) => readItems().filter((i) => i.type === type),
    [],
  )

  return { push, getAll, getByType }
}
