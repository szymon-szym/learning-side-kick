<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Ref, ref } from "vue";

interface Transcription {
    text: string;
    is_final: boolean;
}

const isRecording: Ref<boolean> = ref(false);
const currentTranscription: Ref<string> = ref("");

async function startRecording() {
    await invoke("start_transcription");
    isRecording.value = true;
    currentTranscription.value = "";
    listen<Transcription>("transcription-stream", (event) => {
        console.log("received the transcription event");
        currentTranscription.value =
            currentTranscription.value + "" + event.payload.text;
        isRecording.value = !event.payload.is_final;
    });
}

async function stopRecording() {
    await invoke("stop_recording");
}
</script>

<template>
    <main class="container">
        <h1>Side-kick</h1>
        <div>is recording {{ isRecording }}</div>
        <button v-if="!isRecording" @click="startRecording">
            Zacznij nagrywać
        </button>
        <button v-if="isRecording" @click="stopRecording">
            Zatrzymaj nagrywanie
        </button>
        <div>
            <p>transcription</p>
            {{ currentTranscription }}
        </div>
    </main>
</template>

<style></style>
