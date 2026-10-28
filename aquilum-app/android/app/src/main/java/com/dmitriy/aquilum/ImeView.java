package com.dmitriy.aquilum;

import android.content.Context;
import android.text.Editable;
import android.text.InputType;
import android.text.Selection;
import android.view.KeyEvent;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

import androidx.core.view.inputmethod.EditorInfoCompat;

import java.util.Arrays;

final class ImeView extends View {
    private static native void onImeState(int session, int seq, boolean echo, char[] text, int[] marks);

    private final InputMethodManager keyboard;
    private final Editable text = Editable.Factory.getInstance().newEditable("");
    private final Connection connection = new Connection();
    private boolean active;
    private boolean multiline;
    private int session;
    private int base = -1;
    private int sent;
    private int batch;
    private String lastText;
    private int[] lastMarks;

    ImeView(Context context) {
        super(context);
        setFocusable(true);
        setFocusableInTouchMode(true);
        keyboard = context.getSystemService(InputMethodManager.class);
    }

    void start(boolean multiline, int session) {
        this.multiline = multiline;
        this.session = session;
        active = true;
        base = -1;
        batch = 0;
        lastText = null;
        text.replace(0, text.length(), "");
        requestFocus();
        keyboard.restartInput(this);
        keyboard.showSoftInput(this, 0);
    }

    void stop() {
        active = false;
        batch = 0;
        keyboard.hideSoftInputFromWindow(getWindowToken(), 0);
        clearFocus();
        keyboard.restartInput(this);
    }

    void show() {
        if (active) {
            requestFocus();
            keyboard.showSoftInput(this, 0);
        }
    }

    void apply(int applied, int base, String value, int anchor, int cursor, int start, int end) {
        int[] marks = {anchor, cursor, start, end, base};
        if (!active || applied < sent || batch > 0 || value.equals(lastText) && Arrays.equals(marks, lastMarks)) {
            return;
        }
        int composing = BaseInputConnection.getComposingSpanStart(text);
        boolean restart = base != this.base || composing >= 0 && (composing != start || BaseInputConnection.getComposingSpanEnd(text) != end);
        batch++;
        text.replace(0, text.length(), value);
        Selection.setSelection(text, anchor, cursor);
        if (start >= 0) {
            connection.setComposingRegion(start, end);
        } else {
            BaseInputConnection.removeComposingSpans(text);
        }
        batch--;
        this.base = base;
        send(true);
        if (restart) {
            keyboard.restartInput(this);
        } else {
            keyboard.updateSelection(this, anchor, cursor, start, end);
        }
    }

    @Override
    public boolean onCheckIsTextEditor() {
        return active;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo info) {
        if (!active) {
            return null;
        }
        info.inputType = multiline
                ? InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_CAP_SENTENCES | InputType.TYPE_TEXT_FLAG_AUTO_CORRECT
                : InputType.TYPE_CLASS_TEXT;
        info.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN | (multiline ? EditorInfo.IME_FLAG_NO_ENTER_ACTION : EditorInfo.IME_ACTION_DONE);
        info.initialSelStart = Selection.getSelectionStart(text);
        info.initialSelEnd = Selection.getSelectionEnd(text);
        EditorInfoCompat.setInitialSurroundingText(info, text);
        return connection;
    }

    private boolean edited(boolean done) {
        if (batch == 0) {
            send(false);
        }
        return done;
    }

    private void send(boolean echo) {
        String value = text.toString();
        int[] marks = {Selection.getSelectionStart(text), Selection.getSelectionEnd(text),
                BaseInputConnection.getComposingSpanStart(text), BaseInputConnection.getComposingSpanEnd(text), base};
        if (!value.equals(lastText) || !Arrays.equals(marks, lastMarks)) {
            lastText = value;
            lastMarks = marks;
            onImeState(session, ++sent, echo, value.toCharArray(), marks);
        }
    }

    private final class Connection extends BaseInputConnection {
        Connection() {
            super(ImeView.this, true);
        }

        @Override
        public Editable getEditable() {
            return text;
        }

        @Override
        public boolean beginBatchEdit() {
            batch++;
            return true;
        }

        @Override
        public boolean endBatchEdit() {
            batch = Math.max(0, batch - 1);
            return edited(batch > 0);
        }

        @Override
        public boolean commitText(CharSequence value, int position) {
            return edited(super.commitText(value, position));
        }

        @Override
        public boolean setComposingText(CharSequence value, int position) {
            return edited(super.setComposingText(value, position));
        }

        @Override
        public boolean setComposingRegion(int start, int end) {
            return edited(super.setComposingRegion(start, end));
        }

        @Override
        public boolean finishComposingText() {
            return edited(super.finishComposingText());
        }

        @Override
        public boolean deleteSurroundingText(int before, int after) {
            return edited(super.deleteSurroundingText(before, after));
        }

        @Override
        public boolean deleteSurroundingTextInCodePoints(int before, int after) {
            return edited(super.deleteSurroundingTextInCodePoints(before, after));
        }

        @Override
        public boolean setSelection(int start, int end) {
            return edited(super.setSelection(start, end));
        }

        @Override
        public boolean sendKeyEvent(KeyEvent event) {
            if (multiline && event.getKeyCode() == KeyEvent.KEYCODE_ENTER) {
                if (event.getAction() == KeyEvent.ACTION_DOWN) {
                    beginBatchEdit();
                    finishComposingText();
                    commitText("\n", 1);
                    endBatchEdit();
                }
                return true;
            }
            send(false);
            return super.sendKeyEvent(event);
        }
    }
}
