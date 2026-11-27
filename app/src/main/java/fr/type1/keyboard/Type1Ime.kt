package fr.type1.keyboard

import android.content.Intent
import android.content.SharedPreferences
import android.database.ContentObserver
import android.icu.text.BreakIterator
import android.inputmethodservice.InputMethodService
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.Settings
import android.provider.UserDictionary
import android.view.KeyEvent
import android.view.View
import android.view.WindowInsets
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import fr.type1.keyboard.core.KeyboardController
import fr.type1.keyboard.core.PersonalDictionary
import fr.type1.keyboard.model.KeyAction
import java.util.concurrent.Executors
import kotlin.math.abs

/**
 * The IME itself. All keyboard logic lives in the Rust core; this service only
 * feeds editor information down and applies the resulting actions upwards.
 */
class Type1Ime : InputMethodService(), KeyboardView.Listener {

	private val controller = KeyboardController()
	private val personalDictionaryExecutor = Executors.newSingleThreadExecutor()
	private var personalDictionaryObserver: ContentObserver? = null
	private var keyboardView: KeyboardView? = null
	private var selectAnchor = -1
	private var selectWidth = 0
	private var lastActionAt = 0L
	private var expectedSelStart = -1
	private var expectedSelEnd = -1
	private var allowNextSelection = false
	private val profileListener = SharedPreferences.OnSharedPreferenceChangeListener { _, key ->
		if (key == Prefs.PROFILES || key == Prefs.ACTIVE_PROFILE) {
			keyboardView?.post {
				applyProfile()
				updateProfileAvailability()
				syncContextFromEditor()
			}
		}
	}

	override fun onCreate() {
		super.onCreate()
		Prefs.preferences(this).registerOnSharedPreferenceChangeListener(profileListener)
		val observer = object : ContentObserver(Handler(Looper.getMainLooper())) {
			override fun onChange(selfChange: Boolean, uri: Uri?) {
				refreshPersonalDictionary()
			}
		}
		personalDictionaryObserver = observer
		contentResolver.registerContentObserver(
			UserDictionary.Words.CONTENT_URI,
			true,
			observer,
		)
		refreshPersonalDictionary()
	}

	override fun onDestroy() {
		Prefs.preferences(this).unregisterOnSharedPreferenceChangeListener(profileListener)
		personalDictionaryObserver?.let {
			contentResolver.unregisterContentObserver(it)
		}
		personalDictionaryObserver = null
		personalDictionaryExecutor.shutdownNow()
		super.onDestroy()
	}

	/** Reload the system personal dictionary off the main thread. */
	private fun refreshPersonalDictionary() {
		personalDictionaryExecutor.execute {
			val entries = PersonalDictionary.load(this)
			controller.setPersonalWords(entries)
			keyboardView?.post { keyboardView?.refresh() }
		}
	}

	override fun onCreateInputView(): View {
		val view = KeyboardView(this, controller)
		view.listener = this
		applyPreferences(view)
		applyProfile()
		view.setOnApplyWindowInsetsListener { _, insets ->
			view.setBottomInset(navigationBarHeight(insets))
			insets
		}
		keyboardView = view
		updateProfileAvailability()
		view.refresh()
		return view
	}

	/** Push the settings that can change between keyboard invocations. */
	private fun applyPreferences(view: KeyboardView) {
		view.setSpaceCursorEnabled(Prefs.spaceCursor(this))
		view.setBackspaceHold(Prefs.backspaceHold(this))
		view.setSwipeSensitivity(Prefs.swipeSensitivity(this))
		view.setSuggestionRowHeight(Prefs.suggestionRowHeight(this))
		view.setLetterRowHeight(Prefs.letterRowHeight(this))
		view.setUtilityRowHeight(Prefs.utilityRowHeight(this))
	}

	override fun onStartInput(attribute: EditorInfo?, restarting: Boolean) {
		super.onStartInput(attribute, restarting)
		clearSelectionTracking()
		lastActionAt = 0L
		controller.onStartInput(attribute)
	}

	override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
		super.onStartInputView(info, restarting)
		// Some IME windows do not dispatch insets to the input view; query the
		// window directly as well.
		applyNavigationInset()
		keyboardView?.let { applyPreferences(it) }
		selectAnchor = -1
		selectWidth = 0
		applyProfile()
		refreshPersonalDictionary()
		updateProfileAvailability()
		// Some editors set imeOptions just before the view appears.
		if (info != null) controller.setImeOptions(info.imeOptions)
		keyboardView?.refresh()
		syncContextFromEditor()
	}

	/** Apply the active profile, or the built-in one when none is configured. */
	private fun applyProfile() {
		val profiles = Prefs.profiles(this)
		val activeId = Prefs.activeProfileId(this)
		val profile = profiles.firstOrNull { it.id == activeId } ?: profiles.firstOrNull()
		if (profile == null) {
			controller.setDefaultProfile()
		} else {
			if (profile.id != activeId) {
				Prefs.setActiveProfileId(this, profile.id)
			}
			// A stored layout or language may disappear across versions.
			if (!controller.setProfile(profile.layoutId, profile.languageIds)) {
				controller.setDefaultProfile()
			}
		}
	}

	/** Tell the comma popup whether there is another profile to switch to. */
	private fun updateProfileAvailability() {
		keyboardView?.setProfileCount(Prefs.profiles(this).size)
	}

	override fun onCycleProfile() {
		val profiles = Prefs.profiles(this)
		if (profiles.size < 2) return
		val current = Prefs.activeProfile(this) ?: return
		val index = profiles.indexOfFirst { it.id == current.id }
		val next = profiles[(index + 1) % profiles.size]
		// The preference listener reconfigures the engine and refreshes the
		// keyboard when the active profile changes.
		Prefs.setActiveProfileId(this, next.id)
	}

	override fun onEmoji(emoji: String) {
		val ic = currentInputConnection ?: return
		ic.commitText(emoji, 1)
		// Remember the pick for the recent tab before the panel refreshes it.
		Prefs.addRecentEmoji(this, emoji)
		// Re-read the context once the editor has accepted the emoji.
		keyboardView?.post { syncContextFromEditor() }
	}

	override fun onOpenSettings() {
		requestHideSelf(0)
		val intent = (
			Intent(this, SettingsActivity::class.java)
			. addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
		)
		startActivity(intent)
	}

	override fun onSpaceDragStart(): Pair<Int, Int> {
		val ic = currentInputConnection ?: return 0 to 0
		val before = ic.getTextBeforeCursor(MAX_CURSOR_DRAG_CHARS, 0)
		val after = ic.getTextAfterCursor(MAX_CURSOR_DRAG_CHARS, 0)
		return (before?.graphemeCount() ?: 0) to (after?.graphemeCount() ?: 0)
	}

	/** Number of user-perceived characters in a text window. */
	private fun CharSequence.graphemeCount(): Int {
		if (isEmpty()) return 0
		val iterator = BreakIterator.getCharacterInstance()
		iterator.setText(toString())
		var count = 0
		var boundary = iterator.first()
		while (boundary != BreakIterator.DONE) {
			boundary = iterator.next()
			if (boundary != BreakIterator.DONE) count++
		}
		return count
	}

	override fun onWindowHidden() {
		keyboardView?.dismissEmojiPicker()
		super.onWindowHidden()
	}

	override fun onFinishInput() {
		super.onFinishInput()
		controller.reset()
		clearSelectionTracking()
		selectAnchor = -1
		selectWidth = 0
		keyboardView?.dismissEmojiPicker()
		keyboardView?.refresh()
	}

	private fun clearSelectionTracking() {
		expectedSelStart = -1
		expectedSelEnd = -1
		allowNextSelection = false
	}

	private fun applyNavigationInset() {
		val insets = window?.window?.decorView?.rootWindowInsets
		keyboardView?.setBottomInset(navigationBarHeight(insets))
	}

	private fun navigationBarHeight(insets: WindowInsets?): Float {
		val reported = when {
			insets == null -> 0f
			Build.VERSION.SDK_INT >= Build.VERSION_CODES.R -> insets.getInsets(WindowInsets.Type.navigationBars()).bottom.toFloat()
			else -> {
				@Suppress("DEPRECATION")
				insets.systemWindowInsetBottom.toFloat()
			}
		}
		// Android 15+ lays the IME out edge-to-edge and floats the system IME
		// controls (hide keyboard chevron, keyboard switcher globe) over the
		// bottom of the window. The window often reports a zero navigation bar
		// inset there, so reserve the standard bar height explicitly. With
		// three-button navigation the window is inset normally and nothing is
		// added.
		val gestureNavigation = try {
			Settings.Secure.getInt(contentResolver, "navigation_mode", 2) == 2
		} catch (_: Exception) {
			true
		}
		val edgeToEdge = if (Build.VERSION.SDK_INT >= 35 && gestureNavigation) {
			48f * resources.displayMetrics.density
		} else {
			0f
		}
		return maxOf(reported, edgeToEdge)
	}

	/** Never take over the screen in landscape: the keyboard is already compact. */
	override fun onEvaluateFullscreenMode(): Boolean = false

	override fun onActions(actions: List<KeyAction>) {
		val ic = currentInputConnection ?: return
		val trimAfter = when (val last = actions.lastOrNull()) {
			is KeyAction.Commit -> last.text.endsWith(' ')
			is KeyAction.CommitSuggestion -> true
			else -> false
		}
		var cursorMoved = false
		var resyncContext = false
		// Keep replacement (delete + commit) atomic for the editor.
		ic.beginBatchEdit()
		try {
			for (action in actions) {
				when (action) {
					is KeyAction.Commit -> ic.commitText(action.text, 1)
					is KeyAction.CommitSuggestion -> ic.commitText(action.text, 1)
					is KeyAction.DeleteWord -> {
						resyncContext = deleteWord(ic, action.word)
					}
					is KeyAction.DeleteChars -> if (action.count > 0) {
						ic.deleteSurroundingText(action.count, 0)
					}
					is KeyAction.ReplaceSurrounding -> {
						ic.deleteSurroundingText(action.before, action.after)
						ic.commitText(action.text, 1)
					}
					is KeyAction.Cursor -> {
						sendCursor(ic, action.delta)
						cursorMoved = true
					}
					is KeyAction.StartSelection -> startSelection(ic)
					is KeyAction.ExtendSelection -> extendSelection(ic, action.delta)
					is KeyAction.DeleteSelection -> {
						deleteSelection(ic)
						resyncContext = true
					}
					is KeyAction.Ime -> performEditorAction(ic, action.action)
				}
			}
			if (trimAfter) trimSpacesAfterCursor(ic)
		} finally {
			ic.endBatchEdit()
		}
		if (cursorMoved) {
			// The selection callback for a cursor move is a user-visible move:
			// let it refresh the context word at the new caret.
			expectedSelStart = -1
			expectedSelEnd = -1
			allowNextSelection = true
		} else {
			val cursor = cursorOffset(ic)
			expectedSelStart = cursor
			expectedSelEnd = cursor
			allowNextSelection = false
		}
		lastActionAt = SystemClock.uptimeMillis()
		// Backspace can reveal the word that was just finished; re-read it so
		// its corrections come back without moving the caret. Post it so a
		// DEL key event has reached the editor before the text is read.
		if (resyncContext) keyboardView?.post { syncContextFromEditor() }
	}

	/** Keep only the space just committed when the old text already had one. */
	private fun trimSpacesAfterCursor(ic: InputConnection) {
		var surplus = 0
		while (surplus < 4) {
			val next = ic.getTextAfterCursor(1, 0)?.toString()
			if (next != " " && next != "\u00A0") break
			ic.deleteSurroundingText(0, 1)
			surplus++
		}
	}

	override fun onUpdateSelection(
		oldSelStart: Int,
		oldSelEnd: Int,
		newSelStart: Int,
		newSelEnd: Int,
		candidatesStart: Int,
		candidatesEnd: Int,
	) {
		super.onUpdateSelection(
			oldSelStart,
			oldSelEnd,
			newSelStart,
			newSelEnd,
			candidatesStart,
			candidatesEnd,
		)
		// Ignore the callbacks produced by our own actions, but accept a real
		// caret move immediately, even within the old time window.
		if (allowNextSelection) {
			allowNextSelection = false
			expectedSelStart = -1
			expectedSelEnd = -1
		} else if (expectedSelStart >= 0) {
			val selfInflicted = newSelStart == expectedSelStart && newSelEnd == expectedSelEnd
			expectedSelStart = -1
			expectedSelEnd = -1
			if (selfInflicted) return
		} else if (SystemClock.uptimeMillis() - lastActionAt < SELECTION_IGNORE_MS) {
			return
		}
		if (newSelStart != newSelEnd) {
			controller.selectionChanged()
			keyboardView?.refresh()
			return
		}
		syncContextFromEditor()
	}

	/** Read the text around the caret, its word and its sentence position. */
	private fun syncContextFromEditor() {
		val ic = currentInputConnection
		if (ic == null) {
			controller.selectionChanged()
			keyboardView?.refresh()
			return
		}
		val before = ic.getTextBeforeCursor(MAX_CONTEXT_CHARS, 0)?.toString().orEmpty()
		val after = ic.getTextAfterCursor(MAX_CONTEXT_CHARS, 0)?.toString().orEmpty()
		controller.syncContext(before, after)
		keyboardView?.refresh()
	}

	private fun sendCursor(ic: InputConnection, delta: Int) {
		if (delta == 0) return
		val keyCode = if (delta < 0) KeyEvent.KEYCODE_DPAD_LEFT else KeyEvent.KEYCODE_DPAD_RIGHT
		repeat(abs(delta)) {
			// DPAD at a field edge can move focus to the next/previous view.
			// Stop before that edge: the spacebar gesture only moves the caret.
			val hasRoom = if (delta < 0) {
				!ic.getTextBeforeCursor(1, 0).isNullOrEmpty()
			} else {
				!ic.getTextAfterCursor(1, 0).isNullOrEmpty()
			}
			if (!hasRoom) return
			ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, keyCode))
			ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, keyCode))
		}
	}

	/** The caret offset, or -1 when the editor cannot report the whole prefix. */
	private fun cursorOffset(ic: InputConnection): Int {
		val before = ic.getTextBeforeCursor(MAX_SELECTION_CHARS, 0) ?: return -1
		return if (before.length < MAX_SELECTION_CHARS) before.length else -1
	}

	private fun startSelection(ic: InputConnection) {
		selectAnchor = cursorOffset(ic)
		selectWidth = 0
		if (selectAnchor >= 0) {
			ic.setSelection(selectAnchor, selectAnchor)
		}
	}

	private fun extendSelection(ic: InputConnection, delta: Int) {
		if (selectAnchor < 0) return
		val target = (selectWidth - delta).coerceIn(0, selectAnchor)
		if (target == selectWidth) return
		selectWidth = if (ic.setSelection(selectAnchor - target, selectAnchor)) target else 0
	}

	private fun deleteSelection(ic: InputConnection) {
		if (selectWidth > 0) {
			val now = SystemClock.uptimeMillis()
			ic.sendKeyEvent(KeyEvent(now, now, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_DEL, 0, 0))
			ic.sendKeyEvent(KeyEvent(now, now, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_DEL, 0, 0))
		}
		selectAnchor = -1
		selectWidth = 0
	}

	private fun deleteWord(ic: InputConnection, word: Boolean): Boolean {
		if (!word) {
			return deleteBackward(ic)
		}
		val before = ic.getTextBeforeCursor(64, 0)?.toString().orEmpty()
		val wordStart = before.trimEnd().indexOfLast { it.isWhitespace() } + 1
		val count = before.length - wordStart
		if (count > 0) ic.deleteSurroundingText(count, 0) else ic.deleteSurroundingText(1, 0)
		return true
	}

	/**
	 * Delete one user-perceived character. [InputConnection] counts UTF-16
	 * units, so deleting "one character" would split a surrogate pair (an
	 * emoji) in half. Measure the last grapheme cluster instead, which also
	 * removes skin-tone modifiers, flags and ZWJ sequences as one unit.
	 *
	 * Returns true when the editor context should be re-read: deleting an
	 * emoji, punctuation or space can reveal a word that was already finished.
	 */
	private fun deleteBackward(ic: InputConnection): Boolean {
		val before = ic.getTextBeforeCursor(BACKSPACE_LOOKBEHIND, 0)
		if (before.isNullOrEmpty()) {
			ic.deleteSurroundingText(1, 0)
			return true
		}
		val end = before.length
		val iterator = BreakIterator.getCharacterInstance()
		iterator.setText(before.toString())
		val start = iterator.preceding(end).coerceIn(0, end - 1)
		val count = end - start
		val wasWord = (start until start + count).all { index ->
			val c = before[index]
			c.isLetter() || c == '\'' || c == '’'
		}
		ic.deleteSurroundingText(count, 0)
		return !wasWord
	}

	private fun performEditorAction(ic: InputConnection, action: String) {
		val id = when (action) {
			"go" -> EditorInfo.IME_ACTION_GO
			"search" -> EditorInfo.IME_ACTION_SEARCH
			"send" -> EditorInfo.IME_ACTION_SEND
			"next" -> EditorInfo.IME_ACTION_NEXT
			"done" -> EditorInfo.IME_ACTION_DONE
			"previous" -> EditorInfo.IME_ACTION_PREVIOUS
			else -> EditorInfo.IME_ACTION_NONE
		}
		if (id == EditorInfo.IME_ACTION_NONE) {
			ic.commitText("\n", 1)
		} else {
			ic.performEditorAction(id)
		}
	}

	private companion object {
		const val MAX_SELECTION_CHARS = 10_000
		const val MAX_CURSOR_DRAG_CHARS = 10_000
		const val BACKSPACE_LOOKBEHIND = 64
		const val MAX_CONTEXT_CHARS = 64
		const val SELECTION_IGNORE_MS = 200L
	}
}
