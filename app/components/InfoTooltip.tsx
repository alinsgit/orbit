import { Info } from 'lucide-react'

interface InfoTooltipProps {
  content: React.ReactNode
}

export function InfoTooltip({ content }: InfoTooltipProps) {
  return (
    <div className="group relative inline-flex">
      <Info
        size={16}
        className="text-content-muted hover:text-emerald-500 transition-colors cursor-help"
      />
      <div className="pointer-events-none group-hover:pointer-events-auto opacity-0 group-hover:opacity-100 scale-95 group-hover:scale-100 transition-all duration-200 absolute left-6 top-0 z-50">
        <div className="max-w-xs bg-surface-raised border border-edge rounded-xl p-4 shadow-xl text-sm">
          {content}
        </div>
      </div>
    </div>
  )
}
