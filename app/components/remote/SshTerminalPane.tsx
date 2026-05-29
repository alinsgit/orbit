import { useXterm } from '../../lib/useXterm'
import {
  sshSpawnTerminal,
  sshWriteTerminal,
  sshResizeTerminal,
  sshCloseTerminal,
} from '../../lib/api'

interface SshTerminalPaneProps {
  connection: string
}

/** Interactive SSH shell for a connection, rendered via the shared xterm hook. */
export function SshTerminalPane({ connection }: SshTerminalPaneProps) {
  const id = `ssh-${connection}`
  const { containerRef } = useXterm({
    id,
    spawn: (sid, cols, rows) => sshSpawnTerminal(connection, sid, cols, rows),
    write: sshWriteTerminal,
    resize: sshResizeTerminal,
    close: sshCloseTerminal,
    outputEvent: (sid) => `ssh-pty-output-${sid}`,
    closedEvent: (sid) => `ssh-pty-closed-${sid}`,
  })

  return (
    <div className="h-full bg-[#0d1117]">
      <div ref={containerRef} className="w-full h-full p-2" />
    </div>
  )
}
