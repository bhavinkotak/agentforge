import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Sidebar } from '@/components/layout/Sidebar'
import { TopBar } from '@/components/layout/TopBar'
import { DashboardPage } from '@/pages/DashboardPage'
import { AgentsPage } from '@/pages/AgentsPage'
import { CreateAgentPage } from '@/pages/CreateAgentPage'
import { AgentDetailPage } from '@/pages/AgentDetailPage'
import { RunDetailPage } from '@/pages/RunDetailPage'
import { DiffPage } from '@/pages/DiffPage'
import { ShadowRunDetailPage } from '@/pages/ShadowRunDetailPage'
import { ExportDetailPage } from '@/pages/ExportDetailPage'
import { BenchmarkDetailPage } from '@/pages/BenchmarkDetailPage'

export default function App() {
  return (
    <BrowserRouter>
      <div className="flex h-screen overflow-hidden">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <TopBar />
          <main className="flex-1 overflow-y-auto p-6">
            <Routes>
              <Route path="/" element={<DashboardPage />} />
              <Route path="/agents" element={<AgentsPage />} />
              <Route path="/agents/new" element={<CreateAgentPage />} />
              <Route path="/agents/:id" element={<AgentDetailPage />} />
              <Route path="/runs/:id" element={<RunDetailPage />} />
              <Route path="/diff" element={<DiffPage />} />
              <Route path="/shadow-runs/:id" element={<ShadowRunDetailPage />} />
              <Route path="/exports/finetune/:id" element={<ExportDetailPage />} />
              <Route path="/benchmarks/:id" element={<BenchmarkDetailPage />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </main>
        </div>
      </div>
    </BrowserRouter>
  )
}
