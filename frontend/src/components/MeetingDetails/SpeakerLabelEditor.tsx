"use client";

import { useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Pencil } from 'lucide-react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

interface SpeakerLabelEditorProps {
  meetingId: string;
  speakerId: string;
  speakerLabel: string;
  speakerColor?: string | null;
  diarizationStatus?: string | null;
  onRenamed: () => Promise<void>;
}

function hasControlCharacter(value: string): boolean {
  return Array.from(value).some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 31 || codePoint === 127;
  });
}

export function SpeakerLabelEditor({
  meetingId,
  speakerId,
  speakerLabel,
  speakerColor,
  diarizationStatus,
  onRenamed,
}: SpeakerLabelEditorProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [draftLabel, setDraftLabel] = useState(speakerLabel);
  const labelButtonRef = useRef<HTMLButtonElement>(null);
  const currentIdentityRef = useRef(`${meetingId}:${speakerId}`);
  currentIdentityRef.current = `${meetingId}:${speakerId}`;

  const restoreLabelFocus = () => {
    requestAnimationFrame(() => labelButtonRef.current?.focus());
  };

  const cancelEditing = () => {
    if (isSaving) return;
    setDraftLabel(speakerLabel);
    setIsEditing(false);
    restoreLabelFocus();
  };

  const saveLabel = async () => {
    if (isSaving) return;

    const normalizedLabel = draftLabel.trim();
    if (
      normalizedLabel.length === 0 ||
      Array.from(normalizedLabel).length > 128 ||
      hasControlCharacter(normalizedLabel)
    ) {
      toast.error('Speaker name must contain 1 to 128 printable characters');
      return;
    }

    if (normalizedLabel === speakerLabel) {
      cancelEditing();
      return;
    }

    const requestedIdentity = `${meetingId}:${speakerId}`;
    setIsSaving(true);
    try {
      await invoke('rename_speaker', {
        request: {
          meetingId,
          speakerId,
          label: normalizedLabel,
        },
      });

      if (currentIdentityRef.current !== requestedIdentity) return;
      await onRenamed();
      if (currentIdentityRef.current !== requestedIdentity) return;

      setIsEditing(false);
      toast.success('Speaker renamed');
      restoreLabelFocus();
    } catch (error) {
      console.error('Failed to rename speaker:', error);
      if (currentIdentityRef.current === requestedIdentity) {
        toast.error('Failed to rename speaker');
      }
    } finally {
      if (currentIdentityRef.current === requestedIdentity) {
        setIsSaving(false);
      }
    }
  };

  if (isEditing) {
    return (
      <div className="mb-1 flex items-center gap-1.5">
        <span
          aria-hidden="true"
          className="h-2.5 w-2.5 shrink-0 rounded-full border border-border"
          style={{ backgroundColor: speakerColor || '#94a3b8' }}
        />
        <Input
          autoFocus
          aria-label="Speaker name"
          className="h-8 min-w-0 flex-1 text-xs"
          value={draftLabel}
          disabled={isSaving}
          maxLength={128}
          onChange={(event) => setDraftLabel(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              void saveLabel();
            }
            if (event.key === 'Escape') {
              event.preventDefault();
              cancelEditing();
            }
          }}
        />
        <Button size="sm" className="h-8 px-2" disabled={isSaving} onClick={() => void saveLabel()}>
          Save
        </Button>
        <Button size="sm" variant="ghost" className="h-8 px-2" disabled={isSaving} onClick={cancelEditing}>
          Cancel
        </Button>
      </div>
    );
  }

  return (
    <button
      ref={labelButtonRef}
      type="button"
      className="mb-1 flex items-center gap-1.5 rounded px-1 py-0.5 text-xs font-medium text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-info"
      aria-label={`Rename speaker ${speakerLabel}`}
      onClick={() => {
        setDraftLabel(speakerLabel);
        setIsEditing(true);
      }}
    >
      <span
        aria-hidden="true"
        className="h-2.5 w-2.5 rounded-full border border-border"
        style={{ backgroundColor: speakerColor || '#94a3b8' }}
      />
      <span>{speakerLabel}</span>
      {diarizationStatus === 'provisional' && (
        <span className="font-normal text-muted-foreground">Provisional</span>
      )}
      <Pencil aria-hidden="true" className="h-3 w-3 text-muted-foreground" />
    </button>
  );
}
