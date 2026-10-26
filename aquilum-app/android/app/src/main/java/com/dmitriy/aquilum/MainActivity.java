package com.dmitriy.aquilum;

import android.Manifest;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.KeyEvent;
import android.view.View;

import androidx.activity.EdgeToEdge;
import androidx.activity.OnBackPressedCallback;
import androidx.activity.SystemBarStyle;
import androidx.core.graphics.Insets;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;
import androidx.lifecycle.Lifecycle;

import com.google.androidgamesdk.GameActivity;

public class MainActivity extends GameActivity {
    private static final int SAFE_AREA = WindowInsetsCompat.Type.systemBars()
            | WindowInsetsCompat.Type.displayCutout()
            | WindowInsetsCompat.Type.ime();

    private volatile long safeArea;
    private boolean notificationsAsked;

    private static native void onLink(String link);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        EdgeToEdge.enable(this);
        super.onCreate(savedInstanceState);
        getOnBackPressedDispatcher().addCallback(this, new OnBackPressedCallback(true) {
            @Override
            public void handleOnBackPressed() {
                long now = SystemClock.uptimeMillis();
                onKeyDown(KeyEvent.KEYCODE_BACK, new KeyEvent(now, now, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_BACK, 0));
                onKeyUp(KeyEvent.KEYCODE_BACK, new KeyEvent(now, now, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_BACK, 0));
            }
        });
        deliver(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        deliver(intent);
    }

    private static void deliver(Intent intent) {
        Uri link = intent.getData();
        if (Intent.ACTION_VIEW.equals(intent.getAction()) && link != null) {
            onLink(link.toString());
        }
    }

    public void leave() {
        runOnUiThread(() -> moveTaskToBack(true));
    }

    public void setDarkTheme(boolean dark) {
        runOnUiThread(() -> {
            SystemBarStyle style = dark
                    ? SystemBarStyle.dark(Color.TRANSPARENT)
                    : SystemBarStyle.light(Color.TRANSPARENT, Color.TRANSPARENT);
            EdgeToEdge.enable(this, style, style);
            WindowInsetsControllerCompat controller = WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
            controller.setAppearanceLightStatusBars(!dark);
            controller.setAppearanceLightNavigationBars(!dark);
        });
    }

    @Override
    protected void onStop() {
        super.onStop();
        SyncService.keep(this);
    }

    public void syncProgress(String title, int done, int total) {
        runOnUiThread(() -> {
            boolean visible = getLifecycle().getCurrentState().isAtLeast(Lifecycle.State.STARTED);
            SyncService.progress(this, visible, title, done, total);
            if (visible && total > 0) {
                askNotifications();
            }
        });
    }

    public void syncFinished() {
        runOnUiThread(SyncService::finished);
    }

    private void askNotifications() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && !notificationsAsked
                && checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            notificationsAsked = true;
            requestPermissions(new String[] {Manifest.permission.POST_NOTIFICATIONS}, 0);
        }
    }

    @Override
    public WindowInsetsCompat onApplyWindowInsets(View view, WindowInsetsCompat insets) {
        Insets safe = insets.getInsets(SAFE_AREA);
        safeArea = (long) safe.left | (long) safe.top << 16 | (long) safe.right << 32 | (long) safe.bottom << 48;
        return super.onApplyWindowInsets(view, insets);
    }

    public long safeArea() {
        return safeArea;
    }

    public void copyText(String text) {
        ClipboardManager clipboard = getSystemService(ClipboardManager.class);
        clipboard.setPrimaryClip(ClipData.newPlainText("Aquilum", text));
    }

    public String pasteText() {
        ClipData clip = getSystemService(ClipboardManager.class).getPrimaryClip();
        if (clip == null || clip.getItemCount() == 0) {
            return null;
        }
        CharSequence text = clip.getItemAt(0).coerceToText(this);
        return text == null ? null : text.toString();
    }

    public void openUri(String uri) {
        try {
            startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(uri)));
        } catch (ActivityNotFoundException ignored) {
        }
    }
}
