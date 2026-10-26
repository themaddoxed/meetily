# System Architecture

Meetily is a self-contained desktop application built with [Tauri](https://tauri.app/). It combines a Rust-based backend with a Next.js frontend into the macOS application. The packaged UI is a static export, not a development server.

## High-Level Architecture Diagram

```mermaid
graph TD
    subgraph User Interface
        A[Next.js Frontend]
    end

    subgraph "Core Logic (Rust)"
        B[Tauri Core]
        C[Audio Engine]
        D[Transcription Engine]
        E[Database]
        F[Summary Engine]
    end

    A -- Tauri Commands --> B
    B -- Manages --> C
    B -- Manages --> D
    B -- Manages --> E
    B -- Manages --> F
```

## Component Details

### Frontend (Next.js)

*   Provides the user interface for managing meetings, displaying transcriptions, and configuring the application.
*   Communicates with the Rust core through Tauri's command system.

### Backend (Rust Core)

*   **Tauri Core:** The heart of the application, responsible for managing the window, handling events, and exposing the Rust core to the frontend.
*   **Audio Engine:** Captures audio from the microphone and system, processes it, and prepares it for transcription.
*   **Transcription Engine:** Uses local speech-to-text models (Whisper or Parakeet) to transcribe the captured audio. It can be accelerated with a GPU.
*   **Database:** A local SQLite database that stores meeting metadata, transcripts, and summaries.
*   **Summary Engine:** Generates meeting summaries using various Large Language Models (LLMs), including local models via Ollama.

## Provider and historical boundaries

Summary and meeting-Q&A can call a selected remote provider with meeting text.
Local providers run on the device when their models/runtimes are available.
See [data handling](../PRIVACY_POLICY.md). The old FastAPI/vector-database and
Electron images in `docs/assets` are historical illustrations; the current
runtime has no separate Python server, vector-database service or Electron
process. Architecture presence is not installed/native acceptance.
