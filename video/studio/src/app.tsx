import { createRoot } from 'react-dom/client'
import { Film } from './Film'

export function start() {
  createRoot(document.getElementById('root')!).render(<Film />)
}
