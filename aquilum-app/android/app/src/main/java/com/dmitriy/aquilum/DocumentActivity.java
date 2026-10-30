package com.dmitriy.aquilum;

import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;
import android.webkit.MimeTypeMap;

import androidx.activity.result.ActivityResultLauncher;
import androidx.activity.result.contract.ActivityResultContracts;

import com.google.androidgamesdk.GameActivity;

import java.io.FileNotFoundException;
import java.util.Arrays;

public abstract class DocumentActivity extends GameActivity {
    private final ActivityResultLauncher<Intent> documents = registerForActivityResult(
            new ActivityResultContracts.StartActivityForResult(),
            result -> onDocument(result.getResultCode() == RESULT_OK && result.getData() != null ? result.getData().getDataString() : null));

    private static native void onDocument(String uri);

    public void pickDocument(String extensions) {
        String[] types = Arrays.stream(extensions.split(" ")).map(DocumentActivity::mimeType).distinct().toArray(String[]::new);
        ask(new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").putExtra(Intent.EXTRA_MIME_TYPES, types));
    }

    public void createDocument(String name) {
        String extension = name.substring(name.lastIndexOf('.') + 1);
        ask(new Intent(Intent.ACTION_CREATE_DOCUMENT).setType(mimeType(extension)).putExtra(Intent.EXTRA_TITLE, name));
    }

    public String documentName(String uri) {
        try (Cursor cursor = getContentResolver().query(Uri.parse(uri), new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            return cursor != null && cursor.moveToFirst() ? cursor.getString(0) : null;
        }
    }

    public int openDocument(String uri, String mode) throws FileNotFoundException {
        return getContentResolver().openFileDescriptor(Uri.parse(uri), mode).detachFd();
    }

    public String cacheDir() {
        return getCacheDir().getPath();
    }

    private void ask(Intent intent) {
        runOnUiThread(() -> {
            try {
                documents.launch(intent.addCategory(Intent.CATEGORY_OPENABLE));
            } catch (ActivityNotFoundException e) {
                onDocument(null);
            }
        });
    }

    private static String mimeType(String extension) {
        String type = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
        return type == null ? "application/octet-stream" : type;
    }
}
