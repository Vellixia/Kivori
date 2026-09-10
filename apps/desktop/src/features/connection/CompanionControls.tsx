import { useCallback, useEffect, useRef, useState } from 'react';
import type { ReactElement } from 'react';
import { Button } from '../../components/ui/button';
import { ToggleGroup, ToggleGroupItem } from '../../components/ui/toggle-group';
import { configureCompanion, playMascotAction } from '../../lib/ipc';
import type { MascotAction, MascotPersonality } from '../../lib/ipc/types';
import { strings } from '../../lib/i18n/strings';

const PERSONALITY_KEY = 'kivori.mascot.personality';
const SELF_PLAY_KEY = 'kivori.mascot.selfPlay';
const PERSONALITIES: readonly MascotPersonality[] = ['cozy', 'playful', 'calm'];
const ACTIONS: readonly MascotAction[] = ['greet', 'pet', 'tickle', 'surprise', 'comfort'];

export function currentMascotPersonality(): MascotPersonality {
  if (typeof localStorage === 'undefined') return 'cozy';
  const saved = localStorage.getItem(PERSONALITY_KEY);
  return PERSONALITIES.includes(saved as MascotPersonality) ? (saved as MascotPersonality) : 'cozy';
}

function initialSelfPlay(): boolean {
  return localStorage.getItem(SELF_PLAY_KEY) !== 'false';
}

interface CompanionControlsProps {
  connected: boolean;
  supported: boolean;
}

/** Production companion controls for personality, ambient self-play, and direct social reactions. */
export function CompanionControls({ connected, supported }: CompanionControlsProps): ReactElement {
  const [personality, setPersonality] = useState<MascotPersonality>(currentMascotPersonality);
  const [selfPlay, setSelfPlay] = useState(initialSelfPlay);
  const [actionError, setActionError] = useState(false);
  const [configurationError, setConfigurationError] = useState(false);
  const configurationAttempt = useRef(0);
  const t = strings.companion;

  const applyConfiguration = useCallback(
    (nextPersonality: MascotPersonality, nextSelfPlay: boolean): void => {
      const attempt = ++configurationAttempt.current;
      setConfigurationError(false);
      void configureCompanion(nextPersonality, nextSelfPlay).catch(() => {
        if (configurationAttempt.current === attempt) setConfigurationError(true);
      });
    },
    [],
  );

  useEffect(() => {
    localStorage.setItem(PERSONALITY_KEY, personality);
    localStorage.setItem(SELF_PLAY_KEY, String(selfPlay));
    applyConfiguration(personality, selfPlay);
  }, [personality, selfPlay, applyConfiguration]);

  const available = connected && supported;

  return (
    <section aria-labelledby="companion-heading" className="space-y-4">
      <div>
        <h2 id="companion-heading" className="font-medium">
          {t.heading}
        </h2>
        <p className="mt-1 text-sm text-muted-foreground">{t.description}</p>
      </div>

      <fieldset className="space-y-2">
        <legend className="text-sm font-medium">{t.personality}</legend>
        <ToggleGroup
          aria-label={t.personality}
          value={[personality]}
          onValueChange={(value) => {
            const selected = value[0] as MascotPersonality | undefined;
            if (selected) setPersonality(selected);
          }}
          variant="outline"
          className="flex-wrap"
        >
          {PERSONALITIES.map((candidate) => (
            <ToggleGroupItem key={candidate} value={candidate}>
              {t.personalities[candidate]}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
      </fieldset>

      <Button
        type="button"
        variant={selfPlay ? 'default' : 'outline'}
        aria-pressed={selfPlay}
        onClick={() => setSelfPlay((enabled) => !enabled)}
      >
        {t.selfPlay}: {selfPlay ? t.on : t.off}
      </Button>

      <fieldset className="space-y-2">
        <legend className="text-sm font-medium">{t.actions}</legend>
        <div className="flex flex-wrap gap-2">
          {ACTIONS.map((action) => (
            <Button
              key={action}
              type="button"
              variant="outline"
              disabled={!available}
              onClick={() => {
                setActionError(false);
                void playMascotAction(action).catch(() => setActionError(true));
              }}
            >
              {t.actionLabels[action]}
            </Button>
          ))}
        </div>
      </fieldset>

      {!connected ? <p className="text-sm text-muted-foreground">{t.connect}</p> : null}
      {connected && !supported ? <p className="text-sm text-muted-foreground">{t.update}</p> : null}
      {configurationError ? (
        <div role="alert" className="flex flex-wrap items-center gap-2 text-sm text-destructive">
          <span>{t.configureFailed}</span>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => applyConfiguration(personality, selfPlay)}
          >
            {t.retry}
          </Button>
        </div>
      ) : null}
      {actionError ? (
        <p role="alert" className="text-sm text-destructive">
          {t.failed}
        </p>
      ) : null}
    </section>
  );
}
