"use client";
import { useEffect, useRef, useState } from 'react';
import Link from 'next/link';
import { invoke } from '@tauri-apps/api/core';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { useI18n } from '@/contexts/I18nContext';

interface Configuration { provider: string; model: string; transport: 'local' | 'network'; fingerprint: string }
interface Answer { meetingId: string; configuration: Configuration; content: {
  insufficientEvidence: boolean; claims: { text: string; citations: { transcriptId: string; quote: string; href: string }[] }[];
} }
const knownErrors = ['context_too_large', 'meeting_changed', 'model_configuration_changed', 'invalid_model_answer', 'cancelled', 'generation_timeout', 'request_in_progress'] as const;

export function MeetingQuestions({ meetingId, modelConfig }: { meetingId: string; modelConfig: ModelConfig }) {
  const { t } = useI18n();
  const [configuration, setConfiguration] = useState<Configuration | null>(null);
  const [question, setQuestion] = useState('');
  const [consent, setConsent] = useState(false);
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const active = useRef<{ id: string; meetingId: string } | null>(null);
  const cancel = () => {
    const request = active.current;
    if (!request) return;
    active.current = null;
    setBusy(false);
    setError('cancelled');
    void invoke('api_cancel_meeting_question', { meetingId: request.meetingId, requestId: request.id }).catch(() => {});
  };

  useEffect(() => {
    let disposed = false;
    setConfiguration(null); setConsent(false); setAnswer(null); setError(null); setQuestion(''); setBusy(false);
    void invoke<Configuration>('api_get_meeting_question_configuration').then(value => {
      if (!disposed) setConfiguration(value);
    }).catch(() => { if (!disposed) setError('model_not_configured'); });
    return () => {
      disposed = true;
      const request = active.current;
      active.current = null;
      if (request) void invoke('api_cancel_meeting_question', { meetingId: request.meetingId, requestId: request.id }).catch(() => {});
    };
  }, [meetingId, modelConfig]);

  const ask = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!configuration || busy || !question.trim() || (configuration.transport === 'network' && !consent)) return;
    const request = { id: crypto.randomUUID(), meetingId };
    active.current = request; setBusy(true); setAnswer(null); setError(null);
    try {
      const value = await invoke<Answer>('api_ask_meeting_question', {
        meetingId, question: question.trim(), requestId: request.id,
        configurationFingerprint: configuration.fingerprint, networkConsent: consent,
      });
      if (active.current !== request) return;
      if (value.meetingId !== meetingId || value.configuration.fingerprint !== configuration.fingerprint) {
        setError('model_configuration_changed'); return;
      }
      setAnswer(value);
    } catch (failure) {
      if (active.current === request) {
        const code = typeof failure === 'string' ? failure : 'generation_failed';
        setError(code);
        if (code === 'model_configuration_changed') {
          setConfiguration(null); setConsent(false);
          try {
            const current = await invoke<Configuration>('api_get_meeting_question_configuration');
            if (active.current === request) setConfiguration(current);
          } catch { if (active.current === request) setError('model_not_configured'); }
        }
      }
    } finally {
      if (active.current === request) { active.current = null; setBusy(false); }
    }
  };
  const errorKey = knownErrors.includes(error as typeof knownErrors[number])
    ? `questions.error.${error}` as `questions.error.${typeof knownErrors[number]}` : 'questions.error.generation_failed';

  return <section className="mt-4 max-h-[45vh] shrink-0 overflow-y-auto rounded-lg border border-border bg-card p-4 text-card-foreground" aria-label={t('questions.title')}>
    <h2 className="font-semibold">{t('questions.title')}</h2>
    <p className="mt-1 text-sm text-muted-foreground">{t('questions.scope')}</p>
    {configuration && <p className="mt-2 text-sm" data-testid="question-provider">{configuration.provider} · {configuration.model} · {t(configuration.transport === 'local' ? 'questions.local' : 'questions.network')}</p>}
    <form onSubmit={ask} className="mt-3 space-y-3">
      <label className="block text-sm">{t('questions.question')}
        <textarea value={question} onChange={event => setQuestion(event.target.value)} maxLength={2000} disabled={busy}
          className="mt-1 w-full rounded-md border border-input bg-background p-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" rows={3} />
      </label>
      {configuration?.transport === 'network' && <label className="flex items-start gap-2 text-sm">
        <input type="checkbox" checked={consent} disabled={busy} onChange={event => setConsent(event.target.checked)} />{t('questions.consent')}
      </label>}
      <div className="flex gap-2">
        <button type="submit" disabled={busy || !configuration || !question.trim() || (configuration.transport === 'network' && !consent)} className="rounded-md bg-primary px-3 py-2 text-primary-foreground disabled:opacity-50 focus-visible:ring-2 focus-visible:ring-ring">{t(busy ? 'questions.asking' : 'questions.ask')}</button>
        {busy && configuration && <button type="button" onClick={cancel} className="rounded-md border border-border px-3 py-2 focus-visible:ring-2 focus-visible:ring-ring">{t('questions.cancel')}</button>}
      </div>
    </form>
    {error && <p role="alert" className="mt-3 text-sm text-destructive">{t(error === 'model_not_configured' ? 'questions.unconfigured' : errorKey)}</p>}
    {answer && <div role="status" className="mt-4 space-y-3">
      <p className="text-xs text-muted-foreground">{answer.configuration.provider} · {answer.configuration.model}</p>
      {answer.content.insufficientEvidence ? <p>{t('questions.insufficient')}</p> : <>
        <p className="text-xs text-muted-foreground">{t('questions.reviewEvidence')}</p>
        {answer.content.claims.map((claim, index) => <div key={index}>
          <p className="whitespace-pre-wrap">{claim.text}</p>
          {claim.citations.map((citation, citationIndex) => <blockquote key={`${citation.transcriptId}-${citationIndex}`} className="mt-2 border-l-2 border-border pl-3 text-sm">
            <p className="whitespace-pre-wrap">{citation.quote}</p>
            <Link href={citation.href} className="text-primary underline">{t('questions.openSource')}</Link>
          </blockquote>)}
        </div>)}
      </>}
    </div>}
  </section>;
}
