<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, watch } from "vue";
import { EditorView, basicSetup } from "codemirror";
import { EditorState, Compartment } from "@codemirror/state";
import { json } from "@codemirror/lang-json";
const props = withDefaults(
  defineProps<{ modelValue: string; readonly?: boolean; isJson?: boolean }>(),
  { readonly: false, isJson: false },
);
const emit = defineEmits<{ "update:modelValue": [value: string] }>();
const root = ref<HTMLElement>();
let view: EditorView | undefined;
const language = new Compartment();
const access = new Compartment();
function accessExtensions() {
  return [
    EditorState.readOnly.of(props.readonly),
    EditorView.contentAttributes.of({
      "aria-readonly": String(props.readonly),
      "aria-label": props.readonly ? "响应体或只读请求体" : "请求体",
    }),
  ];
}
onMounted(() => {
  view = new EditorView({
    parent: root.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        basicSetup,
        language.of(props.isJson ? json() : []),
        // Keep the content focusable so native selection, Select All and Copy work.
        // CodeMirror readOnly blocks changes without disabling the editing surface.
        access.of(accessExtensions()),
        EditorView.lineWrapping,
        EditorView.theme({
          "&": { height: "100%", fontSize: "13px" },
          ".cm-scroller": {
            overflow: "auto",
            fontFamily: '"SFMono-Regular", Consolas, monospace',
          },
          ".cm-content": {
            padding: "12px 0 32px",
            userSelect: "text",
            "-webkit-user-select": "text",
          },
          ".cm-gutters": {
            backgroundColor: "#fbfcfd",
            color: "#a3adba",
            border: "none",
          },
          ".cm-activeLine": { backgroundColor: "#f3f7fb" },
          ".cm-activeLineGutter": { backgroundColor: "#f3f7fb" },
          "&.cm-focused": { outline: "none" },
        }),
        EditorView.updateListener.of((update) => {
          if (
            update.docChanged &&
            update.state.doc.toString() !== props.modelValue
          )
            emit("update:modelValue", update.state.doc.toString());
        }),
      ],
    }),
  });
});
watch(
  () => props.modelValue,
  (value) => {
    if (view && view.state.doc.toString() !== value)
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: value },
      });
  },
);
watch(
  () => props.isJson,
  (value) =>
    view?.dispatch({ effects: language.reconfigure(value ? json() : []) }),
);
watch(
  () => props.readonly,
  () => view?.dispatch({ effects: access.reconfigure(accessExtensions()) }),
);
onBeforeUnmount(() => view?.destroy());
</script>
<template><div ref="root" class="code-editor"></div></template>
