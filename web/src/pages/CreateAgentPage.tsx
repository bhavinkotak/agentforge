import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useMutation } from '@tanstack/react-query'
import { Upload, Link as LinkIcon, FileText } from 'lucide-react'
import { createAgent } from '@/api/agents'
import { ApiError } from '@/api/client'
import { Button } from '@/components/ui/Button'
import { Input, Textarea } from '@/components/ui/Input'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card'

const EXAMPLE = `name: my-support-agent
version: "1.0"
model: gpt-4o
system_prompt: |
  You are a helpful customer support agent.
  Always be polite and resolve issues efficiently.
tools:
  - name: lookup_order
    description: Look up order status by order ID
    parameters:
      order_id:
        type: string
        description: The order ID to look up
eval_hints:
  pass_threshold: 0.85
  scenario_count: 50
`

export function CreateAgentPage() {
  const navigate = useNavigate()
  const [mode, setMode] = useState<'paste' | 'url'>('paste')
  const [content, setContent] = useState('')
  const [url, setUrl] = useState('')
  const [fetchError, setFetchError] = useState<string | null>(null)
  const [fetching, setFetching] = useState(false)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  const mutation = useMutation({
    mutationFn: createAgent,
  })

  async function handleFetchUrl() {
    setFetchError(null)
    if (!url.trim()) return
    if (!/^https?:\/\//i.test(url.trim())) {
      setFetchError('URL must start with http:// or https://')
      return
    }
    setFetching(true)
    try {
      const res = await fetch(url.trim())
      if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText}`)
      const text = await res.text()
      setContent(text)
      setMode('paste')
    } catch (e) {
      setFetchError(e instanceof Error ? e.message : 'Failed to fetch URL')
    } finally {
      setFetching(false)
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setErrorMsg(null)
    try {
      const agent = await mutation.mutateAsync({ content })
      navigate(`/agents/${agent.id}`)
    } catch (err) {
      if (err instanceof ApiError) {
        setErrorMsg(err.message)
      } else {
        setErrorMsg('Unexpected error. Check the console.')
      }
    }
  }

  function handleLoadExample() {
    setContent(EXAMPLE)
    setMode('paste')
  }

  return (
    <div className="mx-auto max-w-2xl space-y-5">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Register Agent</h1>
        <p className="mt-0.5 text-sm text-gray-500">
          Provide your agent definition — YAML, JSON, or Markdown (.agent.md with frontmatter).
        </p>
      </div>

      {/* Mode toggle */}
      <div className="flex gap-2">
        <button
          type="button"
          onClick={() => setMode('paste')}
          className={`flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm font-medium transition-colors ${
            mode === 'paste'
              ? 'border-indigo-300 bg-indigo-50 text-indigo-700'
              : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'
          }`}
        >
          <FileText className="h-3.5 w-3.5" />
          Paste content
        </button>
        <button
          type="button"
          onClick={() => setMode('url')}
          className={`flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm font-medium transition-colors ${
            mode === 'url'
              ? 'border-indigo-300 bg-indigo-50 text-indigo-700'
              : 'border-gray-200 bg-white text-gray-600 hover:bg-gray-50'
          }`}
        >
          <LinkIcon className="h-3.5 w-3.5" />
          Fetch from URL
        </button>
      </div>

      {/* URL input */}
      {mode === 'url' && (
        <Card>
          <CardHeader>
            <CardTitle>Agent File URL</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-xs text-gray-500">
              Paste a direct link to a raw agent file — GitHub raw URL, Gist, or any publicly accessible URL returning plain text.
            </p>
            <div className="flex gap-2">
              <Input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://raw.githubusercontent.com/org/repo/main/agent.yaml"
                className="flex-1 font-mono text-xs"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') { e.preventDefault(); void handleFetchUrl() }
                }}
              />
              <Button
                type="button"
                variant="outline"
                onClick={() => void handleFetchUrl()}
                loading={fetching}
              >
                Fetch
              </Button>
            </div>
            {fetchError && (
              <p className="text-xs text-red-600">{fetchError}</p>
            )}
            {content && !fetchError && (
              <p className="text-xs text-green-600">
                ✓ Fetched {content.length.toLocaleString()} characters — review below before registering
              </p>
            )}
          </CardContent>
        </Card>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <Card>
          <CardHeader className="flex items-center justify-between">
            <CardTitle>
              Agent File Content{' '}
              <span className="text-xs font-normal text-gray-400">YAML · JSON · Markdown</span>
            </CardTitle>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleLoadExample}
            >
              Load example
            </Button>
          </CardHeader>
          <CardContent>
            <Textarea
              id="content"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder="Paste your agent YAML, JSON, or Markdown here…"
              rows={20}
              className="font-mono text-xs"
              required
            />
          </CardContent>
        </Card>

        {errorMsg && (
          <div className="rounded-md border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700">
            {errorMsg}
          </div>
        )}

        <div className="flex items-center gap-3">
          <Button type="submit" loading={mutation.isPending}>
            <Upload className="h-4 w-4" />
            Register Agent
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() => navigate('/agents')}
          >
            Cancel
          </Button>
        </div>
      </form>
    </div>
  )
}
