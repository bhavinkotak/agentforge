import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useMutation } from '@tanstack/react-query'
import { Upload } from 'lucide-react'
import { createAgent } from '@/api/agents'
import { ApiError } from '@/api/client'
import { Button } from '@/components/ui/Button'
import { Textarea } from '@/components/ui/Input'
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
  const [content, setContent] = useState('')
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  const mutation = useMutation({
    mutationFn: createAgent,
  })

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
  }

  return (
    <div className="mx-auto max-w-2xl space-y-5">
      <div>
        <h1 className="text-xl font-semibold text-gray-900">Create Agent</h1>
        <p className="mt-0.5 text-sm text-gray-500">
          Paste your agent file content (YAML or JSON) below.
        </p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        <Card>
          <CardHeader className="flex items-center justify-between">
            <CardTitle>Agent File Content</CardTitle>
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
              placeholder="Paste your agent YAML or JSON here…"
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
            Create Agent
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
