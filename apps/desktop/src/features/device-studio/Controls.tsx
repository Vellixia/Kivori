import type { ReactElement } from 'react';
import { mirrorState } from '../../lib/ipc';
import type { CompanionState, SendableState } from '../../lib/ipc/types';
import { COMPANION_STATES, SENDABLE_STATES } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';
import { SCENE_DURATION_MS, STEP_MS, useStudioStore } from './store';

const SENDABLE = new Set<CompanionState>(SENDABLE_STATES);

/** Device Studio control surface: state selection, timeline scrub, transport, and mirror-to-device. */
export function Controls(): ReactElement {
  const state = useStudioStore((s) => s.state);
  const elapsedMs = useStudioStore((s) => s.elapsedMs);
  const playing = useStudioStore((s) => s.playing);
  const setState = useStudioStore((s) => s.setState);
  const seek = useStudioStore((s) => s.seek);
  const step = useStudioStore((s) => s.step);
  const toggle = useStudioStore((s) => s.toggle);

  const t = strings.studio;
  const canMirror = SENDABLE.has(state);

  return (
    <div className="studio-controls">
      <fieldset>
        <legend>{t.stateGroup}</legend>
        <div role="radiogroup" aria-label={t.stateGroup} className="state-group">
          {COMPANION_STATES.map((candidate) => (
            <button
              key={candidate}
              type="button"
              role="radio"
              aria-checked={state === candidate}
              onClick={() => setState(candidate)}
            >
              {t.states[candidate]}
            </button>
          ))}
        </div>
      </fieldset>

      <label className="scrub">
        <span>{t.scrub}</span>
        <input
          type="range"
          min={0}
          max={SCENE_DURATION_MS}
          step={STEP_MS}
          value={elapsedMs}
          onChange={(event) => seek(Number(event.target.value))}
        />
        <output>{elapsedMs}</output>
      </label>

      <div className="transport" role="group" aria-label={t.transport}>
        <button type="button" onClick={toggle} aria-pressed={playing}>
          {playing ? t.pause : t.play}
        </button>
        <button type="button" onClick={() => step()}>
          {t.step}
        </button>
        <button
          type="button"
          disabled={!canMirror}
          onClick={() => {
            if (canMirror) void mirrorState(state as SendableState);
          }}
        >
          {t.mirror}
        </button>
      </div>
    </div>
  );
}
