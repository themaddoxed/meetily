'use client';

import React, { useState, useMemo, useEffect, useCallback, useRef } from 'react';
import { ChevronDown, ChevronRight, File, Settings, ChevronLeftCircle, ChevronRightCircle, Calendar, Home, Trash2, Mic, Square, Plus, Pencil, NotebookPen, SearchIcon, X, Upload } from 'lucide-react';
import { useRouter, usePathname } from 'next/navigation';
import { useSidebar } from './SidebarProvider';
import type { CurrentMeeting, MeetingLifecycleStatus } from '@/components/Sidebar/SidebarProvider';
import { ConfirmationModal } from '../ConfirmationModel/confirmation-modal';
import { invoke } from '@tauri-apps/api/core';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { toast } from 'sonner';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { useConfig } from '@/contexts/ConfigContext';
import { useI18n } from '@/contexts/I18nContext';

import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog"
import { VisuallyHidden } from "@/components/ui/visually-hidden"

import Logo from '../Logo';
import Info from '../Info';
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from '../ui/input-group';
import {
  buildMeetingSearchHref,
  formatMeetingSearchTime,
  groupMeetingSearchResults,
  meetingSearchSourceLabel,
  type MeetingSearchResult,
} from '@/lib/meeting-search';

interface SidebarItem {
  id: string;
  title: string;
  type: 'folder' | 'file';
  children?: SidebarItem[];
  status?: MeetingLifecycleStatus;
  durationSeconds?: number | null;
}

type SettingsWindow = Window & {
  openSettings?: () => void;
};

const Sidebar: React.FC = () => {
  const router = useRouter();
  const pathname = usePathname();
  const {
    currentMeeting,
    setCurrentMeeting,
    sidebarItems,
    isCollapsed,
    toggleCollapse,
    handleRecordingToggle,
    searchTranscripts,
    searchResults,
    isSearching,
    meetings,
    setMeetings
  } = useSidebar();

  // Get recording state from RecordingStateContext (single source of truth)
  const { isRecording } = useRecordingState();
  const { openImportDialog } = useImportDialog();
  const { betaFeatures } = useConfig();
  const { t } = useI18n();
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['meetings']));
  const knownDayFolderIds = useRef<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState<string>('');

  // State for edit modal
  const [editModalState, setEditModalState] = useState<{ isOpen: boolean; meetingId: string | null; currentTitle: string }>({
    isOpen: false,
    meetingId: null,
    currentTitle: ''
  });
  const [editingTitle, setEditingTitle] = useState<string>('');

  // Ensure 'meetings' folder is always expanded
  useEffect(() => {
    if (!expandedFolders.has('meetings')) {
      const newExpanded = new Set(expandedFolders);
      newExpanded.add('meetings');
      setExpandedFolders(newExpanded);
    }
  }, [expandedFolders]);

  // Expand each newly seen day so a freshly recorded or imported meeting is
  // visible immediately. Once a user collapses a day, status refreshes keep
  // that choice intact.
  useEffect(() => {
    const dayFolderIds = sidebarItems
      .find((item) => item.id === 'meetings')
      ?.children?.map((item) => item.id) ?? [];
    setExpandedFolders((previous) => {
      const next = new Set(previous);
      let changed = !next.has('meetings');
      next.add('meetings');
      const known = knownDayFolderIds.current;
      dayFolderIds.forEach((id) => {
        if (!known.has(id) && !next.has(id)) {
          next.add(id);
          changed = true;
        }
      });
      knownDayFolderIds.current = new Set(dayFolderIds);
      return changed ? next : previous;
    });
  }, [sidebarItems]);

  const [deleteModalState, setDeleteModalState] = useState<{ isOpen: boolean; itemId: string | null }>({ isOpen: false, itemId: null });

  // Handle search input changes
  const handleSearchChange = useCallback(async (value: string) => {
    setSearchQuery(value);

    // Empty search returns to the full meeting list and clears stale results.
    if (!value.trim()) {
      await searchTranscripts('');
      return;
    }

    // Search through transcripts
    await searchTranscripts(value);

    // Make sure the meetings folder is expanded when searching
    if (!expandedFolders.has('meetings')) {
      const newExpanded = new Set(expandedFolders);
      newExpanded.add('meetings');
      setExpandedFolders(newExpanded);
    }
  }, [expandedFolders, searchTranscripts]);

  // Combine search results with sidebar items
  const filteredSidebarItems = useMemo(() => {
    if (!searchQuery.trim()) return sidebarItems;

    const matchedMeetingIds = new Set(searchResults.map(result => result.id));
    const normalizedQuery = searchQuery.toLowerCase();
    const filterTree = (item: SidebarItem): SidebarItem | undefined => {
      if (item.type === 'file') {
        return (matchedMeetingIds.has(item.id) || item.title.toLowerCase().includes(normalizedQuery))
          ? item
          : undefined;
      }
      const children = item.children?.map(filterTree).filter((child): child is SidebarItem => !!child) ?? [];
      return children.length > 0 || item.title.toLowerCase().includes(normalizedQuery)
        ? { ...item, children }
        : undefined;
    };
    return sidebarItems.map(filterTree).filter((item): item is SidebarItem => !!item);
  }, [sidebarItems, searchQuery, searchResults]);


  const handleDelete = async (itemId: string) => {
    console.log('Deleting item:', itemId);

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('api_delete_meeting', {
        meetingId: itemId,
      });
      console.log('Meeting deleted successfully');
      const updatedMeetings = meetings.filter((m: CurrentMeeting) => m.id !== itemId);
      setMeetings(updatedMeetings);

      // Show success toast
      toast.success(t('toast.deleteMeetingSuccess'), {
        description: t('toast.deleteMeetingDescription')
      });

      // If deleting the active meeting, navigate to home
      if (currentMeeting?.id === itemId) {
        setCurrentMeeting({ id: 'intro-call', title: '+ New Call' });
        router.push('/');
      }
    } catch (error) {
      console.error('Failed to delete meeting:', error);
      toast.error(t('toast.deleteMeetingFailed'), {
        description: error instanceof Error ? error.message : String(error)
      });
    }
  };

  const handleDeleteConfirm = () => {
    if (deleteModalState.itemId) {
      handleDelete(deleteModalState.itemId);
    }
    setDeleteModalState({ isOpen: false, itemId: null });
  };

  // Handle modal editing of meeting names
  const handleEditStart = (meetingId: string, currentTitle: string) => {
    setEditModalState({
      isOpen: true,
      meetingId: meetingId,
      currentTitle: currentTitle
    });
    setEditingTitle(currentTitle);
  };

  const handleEditConfirm = async () => {
    const newTitle = editingTitle.trim();
    const meetingId = editModalState.meetingId;

    if (!meetingId) return;

    // Prevent empty titles
    if (!newTitle) {
      toast.error(t('toast.emptyMeetingTitle'));
      return;
    }

    try {
      await invoke('api_save_meeting_title', {
        meetingId: meetingId,
        title: newTitle,
      });

      // Update local state
      const updatedMeetings = meetings.map((m: CurrentMeeting) =>
        m.id === meetingId ? { ...m, title: newTitle } : m
      );
      setMeetings(updatedMeetings);

      // Update current meeting if it's the one being edited
      if (currentMeeting?.id === meetingId) {
        setCurrentMeeting({ id: meetingId, title: newTitle });
      }

      toast.success(t('toast.meetingTitleUpdated'));

      // Close modal and reset state
      setEditModalState({ isOpen: false, meetingId: null, currentTitle: '' });
      setEditingTitle('');
    } catch (error) {
      console.error('Failed to update meeting title:', error);
      toast.error(t('toast.meetingTitleUpdateFailed'), {
        description: error instanceof Error ? error.message : String(error)
      });
    }
  };

  const handleEditCancel = () => {
    setEditModalState({ isOpen: false, meetingId: null, currentTitle: '' });
    setEditingTitle('');
  };

  const toggleFolder = (folderId: string) => {
    // Normal toggle behavior for all folders
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(folderId)) {
      newExpanded.delete(folderId);
    } else {
      newExpanded.add(folderId);
    }
    setExpandedFolders(newExpanded);
  };

  // Preserve the optional Rust tray callback without registering a state-only side effect.
  useEffect(() => {
    const settingsWindow = window as SettingsWindow;
    settingsWindow.openSettings = () => undefined;

    // Cleanup on unmount
    return () => {
      delete settingsWindow.openSettings;
    };
  }, []);

  const renderCollapsedIcons = () => {
    if (!isCollapsed) return null;

    const isHomePage = pathname === '/';
    const isMeetingPage = pathname?.includes('/meeting-details');
    const isSettingsPage = pathname === '/settings';

    return (
      <TooltipProvider>
        <div className="flex flex-col items-center space-y-4 mt-4">
          <Logo isCollapsed={isCollapsed} />

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => router.push('/')}
                aria-label={t('nav.home')}
                className={`p-2 rounded-lg transition-colors duration-150 ${isHomePage ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                  }`}
              >
                <Home className="h-5 w-5" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>{t('nav.home')}</p>
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={handleRecordingToggle}
                disabled={isRecording}
                aria-label={isRecording ? t('nav.recordingInProgress') : t('nav.startRecording')}
                className={`p-2 ${isRecording ? 'cursor-not-allowed bg-recording/50' : 'bg-recording hover:bg-recording/90'} rounded-full text-recording-foreground shadow-sm transition-colors duration-150`}
              >
                {isRecording ? (
                  <Square className="h-5 w-5" />
                ) : (
                  <Mic className="h-5 w-5" />
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>{isRecording ? t('nav.recordingInProgress') : t('nav.startRecording')}</p>
            </TooltipContent>
          </Tooltip>

          {betaFeatures.importAndRetranscribe && (
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  onClick={() => openImportDialog()}
                  aria-label={t('nav.importAudio')}
                  className="rounded-lg bg-info/15 p-2 text-info transition-colors duration-150 hover:bg-info/25"
                >
                  <Upload className="h-5 w-5" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>{t('nav.importAudio')}</p>
              </TooltipContent>
            </Tooltip>
          )}

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => {
                  if (isCollapsed) toggleCollapse();
                  toggleFolder('meetings');
                }}
                aria-label={t('nav.meetings')}
                className={`p-2 rounded-lg transition-colors duration-150 ${isMeetingPage ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                  }`}
              >
                <NotebookPen className="h-5 w-5" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>{t('nav.meetings')}</p>
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => router.push('/settings')}
                aria-label={t('nav.settings')}
                className={`p-2 rounded-lg transition-colors duration-150 ${isSettingsPage ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-accent hover:text-accent-foreground'
                  }`}
              >
                <Settings className="h-5 w-5" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>{t('nav.settings')}</p>
            </TooltipContent>
          </Tooltip>

          <Info isCollapsed={isCollapsed} />
        </div>
      </TooltipProvider>
    );
  };

  // Find the preferred source result for a meeting item. The backend can return
  // several source rows for one meeting. Prefer the transcript row so the
  // preview link preserves the exact source anchor when title and transcript
  // text both match the query.
  const searchResultByMeetingId = useMemo(
    () => groupMeetingSearchResults(searchResults),
    [searchResults],
  );

  const findMatchingSnippet = (itemId: string): MeetingSearchResult | null => {
    if (!searchQuery.trim() || !searchResults.length) return null;
    return searchResultByMeetingId.get(itemId) ?? null;
  };

  const openSearchResult = (result: MeetingSearchResult) => {
    setCurrentMeeting({ id: result.id, title: result.title });
    router.push(buildMeetingSearchHref(result));
  };

  const formatDuration = (durationSeconds: number | null | undefined) => {
    if (durationSeconds === null || durationSeconds === undefined || !Number.isFinite(durationSeconds)) {
      return null;
    }
    const totalSeconds = Math.max(0, Math.round(durationSeconds));
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;
    return hours > 0
      ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
      : `${minutes}:${String(seconds).padStart(2, '0')}`;
  };

  const statusStyle: Record<string, string> = {
    recording: 'text-recording',
    processing: 'text-info',
    ready: 'text-muted-foreground',
    error: 'text-destructive',
  };
  const statusLabel: Record<MeetingLifecycleStatus, string> = {
    recording: t('meetings.statusRecording'),
    processing: t('meetings.statusProcessing'),
    ready: t('meetings.statusReady'),
    error: t('meetings.statusError'),
  };

  const renderItem = (item: SidebarItem, depth = 0) => {
    const isExpanded = expandedFolders.has(item.id);
    const paddingLeft = `${depth * 12 + 12}px`;
    const isActive = item.type === 'file' && currentMeeting?.id === item.id;
    const isMeetingItem = item.id.includes('-') && !item.id.startsWith('intro-call');

    // Check if this item has a matching visible source result.
    const matchingResult = isMeetingItem ? findMatchingSnippet(item.id) : null;
    const hasSearchMatch = !!matchingResult;

    if (isCollapsed) return null;

    return (
      <div key={item.id}>
        <div
          className={`group flex items-center transition-[color,background-color,box-shadow] duration-150 ${item.type === 'folder' && depth === 0
            ? 'p-3 text-lg font-semibold h-10 mx-3 mt-3 rounded-lg'
            : `px-3 py-2 my-0.5 rounded-md text-sm ${isActive ? 'bg-info/15 text-info font-medium' :
              hasSearchMatch ? 'bg-warning/15' : 'hover:bg-accent'
            } cursor-pointer`
            }`}
          style={item.type === 'folder' && depth === 0 ? {} : { paddingLeft }}
          onClick={() => {
            if (item.type === 'folder') {
              toggleFolder(item.id);
            } else {
              if (matchingResult) {
                openSearchResult(matchingResult);
                return;
              }

              setCurrentMeeting({ id: item.id, title: item.title });
              const basePath = item.id.startsWith('intro-call') ? '/' :
                item.id.includes('-') ? `/meeting-details?id=${item.id}` : `/notes/${item.id}`;
              router.push(basePath);
            }
          }}
        >
          {item.type === 'folder' ? (
            <>
              {item.id === 'meetings' ? (
                <Calendar className="w-4 h-4 mr-2" />
              ) : item.id === 'notes' ? (
                <Calendar className="w-4 h-4 mr-2" />
              ) : null}
              {item.id === 'meetings' ? (
                <span className={depth === 0 ? "" : "font-medium"}>{t('nav.meetings')}</span>
              ) : (
                <span className="font-medium text-sm">{item.title}</span>
              )}
              <div className="ml-auto">
                {isExpanded ? (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 text-muted-foreground" />
                )}
              </div>
              {searchQuery && item.id === 'meetings' && isSearching && (
                <span className="ml-2 text-xs text-info animate-pulse">{t('nav.searching')}</span>
              )}
            </>
          ) : (
            <div className="flex flex-col w-full">
              <div className="flex items-center w-full">
                {isMeetingItem ? (
                  <div className="mr-2 flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full bg-muted">
                    <File className="h-3.5 w-3.5 text-muted-foreground" />
                  </div>
                ) : (
                  <div className="flex-shrink-0 flex items-center justify-center w-6 h-6 rounded-full mr-2 bg-info/15">
                    <Plus className="w-3.5 h-3.5 text-info" />
                  </div>
                )}
                <div className="min-w-0 flex-1">
                  <span className="block break-words">{item.title}</span>
                  {isMeetingItem && (
                    <div className="mt-0.5 flex items-center gap-2 text-[11px]" data-testid={`meeting-meta-${item.id}`}>
                      <span className={statusStyle[item.status ?? 'ready'] ?? statusStyle.ready} data-testid={`meeting-status-${item.id}`}>
                        {statusLabel[item.status ?? 'ready'] ?? statusLabel.ready}
                      </span>
                      {formatDuration(item.durationSeconds) && (
                        <span className="text-muted-foreground" data-testid={`meeting-duration-${item.id}`}>
                          {formatDuration(item.durationSeconds)}
                        </span>
                      )}
                    </div>
                  )}
                </div>
                {isMeetingItem && (
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleEditStart(item.id, item.title);
                      }}
                      className="flex-shrink-0 rounded-md p-1 hover:bg-info/10 hover:text-info"
                      aria-label={t('nav.editMeetingTitle')}
                    >
                      <Pencil className="w-4 h-4" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setDeleteModalState({ isOpen: true, itemId: item.id });
                      }}
                      className="flex-shrink-0 rounded-md p-1 hover:bg-destructive/10 hover:text-destructive"
                      aria-label={t('nav.deleteMeeting')}
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                )}
              </div>

              {/* Show the visible source snippet and its time if available */}
              {matchingResult && (
                <button
                  type="button"
                  className="mt-1 ml-8 block w-[calc(100%-2rem)] rounded border border-warning/30 bg-warning/10 p-1.5 text-left text-xs text-muted-foreground hover:bg-warning/15"
                  onClick={(event) => {
                    event.stopPropagation();
                    openSearchResult(matchingResult);
                  }}
                  aria-label={`Open ${meetingSearchSourceLabel(matchingResult.sourceType)} match in ${matchingResult.title}`}
                >
                  <span className="font-medium text-warning">
                    {meetingSearchSourceLabel(matchingResult.sourceType)} · {formatMeetingSearchTime(matchingResult)}:
                  </span>{' '}
                  <span className="line-clamp-2">{matchingResult.matchContext}</span>
                </button>
              )}
            </div>
          )}
        </div>
        {item.type === 'folder' && isExpanded && item.children && (
          <div className="ml-1">
            {item.children.map(child => renderItem(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="fixed top-0 left-0 h-screen z-40">
      {/* Floating collapse button */}
      <button
        onClick={toggleCollapse}
        aria-label={isCollapsed ? t('nav.expandSidebar') : t('nav.collapseSidebar')}
        className="absolute -right-6 top-20 z-50 rounded-full border border-border bg-card p-1 text-card-foreground shadow-lg hover:bg-accent"
        style={{ transform: 'translateX(50%)' }}
      >
        {isCollapsed ? (
          <ChevronRightCircle className="w-6 h-6" />
        ) : (
          <ChevronLeftCircle className="w-6 h-6" />
        )}
      </button>

      <div
        data-testid="app-sidebar"
        className={`flex h-screen flex-col border-r border-border bg-card text-card-foreground shadow-sm transition-[width] duration-300 ${isCollapsed ? 'w-16' : 'w-64'
          }`}
      >
        {/*  Header with traffic light spacing */}
        <div className="flex-shrink-0 h-22 flex items-center">

          {/* Title container */}



          <div className="flex-1">
            {!isCollapsed && (
              <div className="p-3">
                <Logo isCollapsed={isCollapsed} />

                <div className="relative mb-1">
                  <InputGroup >
                    <InputGroupInput placeholder={t('nav.searchMeetingContent')} value={searchQuery}
                      onChange={(e) => handleSearchChange(e.target.value)}
                    />
                    <InputGroupAddon>
                      <SearchIcon />
                    </InputGroupAddon>
                    {searchQuery &&
                      <InputGroupAddon align={'inline-end'}>
                        <InputGroupButton
                          onClick={() => handleSearchChange('')}
                        >
                          <X />
                        </InputGroupButton>
                      </InputGroupAddon>
                    }
                  </InputGroup>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Main content - scrollable area */}
        <div className="flex-1 flex flex-col min-h-0">
          {/* Fixed navigation items */}
          <div className="flex-shrink-0">
            {!isCollapsed && (
              <div
                onClick={() => router.push('/')}
                className="mx-3 mt-3 flex h-10 cursor-pointer items-center rounded-lg p-3 text-lg font-semibold hover:bg-accent"
              >
                <Home className="w-4 h-4 mr-2" />
                <span>{t('nav.home')}</span>
              </div>
            )}
          </div>

          {/* Content area */}
          <div className="flex-1 flex flex-col min-h-0">
            {renderCollapsedIcons()}
            {/* Meeting Notes folder header - fixed */}
            {!isCollapsed && (
              <div className="flex-shrink-0">
                {filteredSidebarItems.filter(item => item.type === 'folder').map(item => (
                  <div key={item.id}>
                    <div
                      className="mx-3 mt-3 flex h-10 items-center rounded-lg p-3 text-lg font-semibold transition-colors duration-150"
                    >
                      <NotebookPen className="mr-2 h-4 w-4 text-muted-foreground" />
                      <span className="text-foreground">{item.id === 'meetings' ? t('nav.meetings') : item.title}</span>
                      {searchQuery && item.id === 'meetings' && isSearching && (
                        <span className="ml-2 text-xs text-info animate-pulse">{t('nav.searching')}</span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Scrollable meeting items */}
            {!isCollapsed && (
              <div className="flex-1 overflow-y-auto custom-scrollbar min-h-0">
                {filteredSidebarItems
                  .filter(item => item.type === 'folder' && expandedFolders.has(item.id) && item.children)
                  .map(item => (
                    <div key={`${item.id}-children`} className="mx-3">
                      {item.children!.map(child => renderItem(child, 1))}
                    </div>
                  ))}
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        {!isCollapsed && (

          <div className="flex-shrink-0 border-t border-border p-2">
            <button
              onClick={handleRecordingToggle}
              disabled={isRecording}
              className={`flex w-full items-center justify-center rounded-lg px-3 py-2 text-sm font-medium text-recording-foreground shadow-sm transition-colors ${isRecording ? 'cursor-not-allowed bg-recording/50' : 'bg-recording hover:bg-recording/90'}`}
            >
              {isRecording ? (
                <>
                  <Square className="w-4 h-4 mr-2" />
                  <span>{t('nav.recordingInProgress')}</span>
                </>
              ) : (
                <>
                  <Mic className="w-4 h-4 mr-2" />
                  <span>{t('nav.startRecording')}</span>
                </>
              )}
            </button>

            {betaFeatures.importAndRetranscribe && (
              <button
                onClick={() => openImportDialog()}
                className="mt-1 flex w-full items-center justify-center rounded-lg bg-info px-3 py-2 text-sm font-medium text-info-foreground shadow-sm transition-colors hover:bg-info/90"
              >
                <Upload className="w-4 h-4 mr-2" />
                <span>{t('nav.importAudio')}</span>
              </button>
            )}

            <button
              onClick={() => router.push('/settings')}
              className="mb-1 mt-1 flex w-full items-center justify-center rounded-lg bg-muted px-3 py-1.5 text-sm font-medium text-foreground shadow-sm transition-colors hover:bg-accent"
            >
              <Settings className="w-4 h-4 mr-2" />
              <span>{t('nav.settings')}</span>
            </button>
            <Info isCollapsed={isCollapsed} />
            <div className="flex w-full items-center justify-center px-3 py-1 text-xs text-muted-foreground">
              v0.4.1
            </div>
          </div>
        )}
      </div>

      {/* Confirmation Modal for Delete */}
      <ConfirmationModal
        isOpen={deleteModalState.isOpen}
        text={t('nav.deleteConfirm')}
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteModalState({ isOpen: false, itemId: null })}
      />

      {/* Edit Meeting Title Modal */}
      <Dialog open={editModalState.isOpen} onOpenChange={(open) => {
        if (!open) handleEditCancel();
      }}>
        <DialogContent className="sm:max-w-[425px]">
          <VisuallyHidden>
            <DialogTitle>{t('nav.editTitle')}</DialogTitle>
          </VisuallyHidden>
          <div className="py-4">
            <h3 className="text-lg font-semibold mb-4">{t('nav.editTitle')}</h3>
            <div className="space-y-4">
              <div>
                <label htmlFor="meeting-title" className="mb-2 block text-sm font-medium text-foreground">
                  {t('nav.meetingTitle')}
                </label>
                <input
                  id="meeting-title"
                  type="text"
                  value={editingTitle}
                  onChange={(e) => setEditingTitle(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      handleEditConfirm();
                    } else if (e.key === 'Escape') {
                      handleEditCancel();
                    }
                  }}
                  className="w-full rounded-md border border-input bg-background px-3 py-2 text-foreground placeholder:text-muted-foreground focus:border-transparent focus:outline-none focus:ring-2 focus:ring-ring"
                  placeholder={t('nav.enterMeetingTitle')}
                  autoFocus
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <button
              onClick={handleEditCancel}
              className="rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-accent"
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={handleEditConfirm}
              className="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
            >
              {t('common.save')}
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default Sidebar;
