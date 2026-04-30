import { useLocation, Link } from 'react-router-dom'
import { ChevronRight } from 'lucide-react'

const PATH_LABELS: Record<string, string> = {
  '': 'Dashboard',
  agents: 'Agents',
  runs: 'Runs',
  diff: 'Diff',
  'shadow-runs': 'Shadow Runs',
  exports: 'Exports',
  finetune: 'Fine-tune',
  benchmarks: 'Benchmarks',
  new: 'New',
}

function useBreadcrumbs() {
  const { pathname } = useLocation()
  const segments = pathname.split('/').filter(Boolean)
  return segments.map((seg, i) => {
    const path = '/' + segments.slice(0, i + 1).join('/')
    const label = PATH_LABELS[seg] ?? seg.slice(0, 8) + '…'
    return { path, label }
  })
}

export function TopBar() {
  const crumbs = useBreadcrumbs()

  return (
    <header className="flex h-14 flex-shrink-0 items-center border-b border-gray-200 bg-white px-6 gap-2">
      <Link to="/" className="text-sm text-gray-500 hover:text-gray-900">
        Home
      </Link>
      {crumbs.map(({ path, label }) => (
        <span key={path} className="flex items-center gap-2">
          <ChevronRight className="h-3.5 w-3.5 text-gray-400" />
          <Link to={path} className="text-sm text-gray-700 hover:text-gray-900">
            {label}
          </Link>
        </span>
      ))}
    </header>
  )
}
