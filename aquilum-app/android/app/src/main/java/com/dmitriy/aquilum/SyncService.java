package com.dmitriy.aquilum;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.os.IBinder;
import android.os.PowerManager;
import android.os.SystemClock;
import android.util.Log;

public class SyncService extends Service {
    private static final int NOTIFICATION = 1;
    private static final String CHANNEL = "sync";
    private static final long SHOW_EVERY = 1000;
    private static final long AWAKE_LIMIT = 6 * 60 * 60 * 1000L;

    private static String title = "Sync";
    private static int done;
    private static int total;
    private static boolean syncing;
    private static SyncService running;
    private static long shownAt;

    private PowerManager.WakeLock awake;
    private int lastStart;

    static void progress(Context context, boolean visible, String name, int doneNow, int totalNow) {
        title = name;
        done = doneNow;
        total = totalNow;
        if (!syncing) {
            syncing = true;
            if (!visible) {
                keep(context);
            }
        } else if (running != null && SystemClock.uptimeMillis() - shownAt >= SHOW_EVERY) {
            running.show();
        }
    }

    static void keep(Context context) {
        if (syncing) {
            allowed(() -> context.startForegroundService(new Intent(context, SyncService.class)));
        }
    }

    static void finished() {
        syncing = false;
        if (running != null) {
            running.stop();
        }
    }

    private static boolean allowed(Runnable start) {
        try {
            start.run();
            return true;
        } catch (IllegalStateException refused) {
            Log.w("Aquilum", "служба синхронизации не запущена", refused);
            return false;
        }
    }

    @Override
    public void onCreate() {
        super.onCreate();
        String channelName = (title != null && !title.isEmpty()) ? title : "Sync";
        getSystemService(NotificationManager.class).createNotificationChannel(new NotificationChannel(CHANNEL, channelName, NotificationManager.IMPORTANCE_LOW));
        awake = getSystemService(PowerManager.class).newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "aquilum:sync");
        awake.setReferenceCounted(false);
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        lastStart = startId;
        if (allowed(() -> startForeground(NOTIFICATION, notification())) && syncing) {
            running = this;
            shownAt = SystemClock.uptimeMillis();
            awake.acquire(AWAKE_LIMIT);
        } else {
            stop();
        }
        return START_NOT_STICKY;
    }

    @Override
    public void onTimeout(int startId, int fgsType) {
        stop();
    }

    @Override
    public void onDestroy() {
        awake.release();
        if (running == this) {
            running = null;
        }
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) {
        return null;
    }

    private void stop() {
        running = null;
        stopForeground(STOP_FOREGROUND_REMOVE);
        stopSelf(lastStart);
    }

    private void show() {
        shownAt = SystemClock.uptimeMillis();
        getSystemService(NotificationManager.class).notify(NOTIFICATION, notification());
    }

    private Notification notification() {
        Intent open = new Intent(this, MainActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        return new Notification.Builder(this, CHANNEL)
                .setSmallIcon(android.R.drawable.stat_notify_sync)
                .setContentTitle(title)
                .setContentText(total > 0 ? done + " / " + total : null)
                .setProgress(total, done, total == 0)
                .setContentIntent(PendingIntent.getActivity(this, 0, open, PendingIntent.FLAG_IMMUTABLE))
                .setCategory(Notification.CATEGORY_PROGRESS)
                .setOngoing(true)
                .setOnlyAlertOnce(true)
                .build();
    }
}
