package fr.type1.keyboard

import android.inputmethodservice.InputMethodService
import android.os.Build
import android.os.SystemClock
import android.provider.Settings
import android.view.KeyEvent
import android.view.View
import android.view.WindowInsets
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import fr.type1.keyboard.core.KeyboardController
import fr.type1.keyboard.model.KeyAction
import kotlin.math.abs

/**
 * The IME itself. All keyboard logic lives in the Rust core; this service only
 * feeds editor information down and applies the resulting actions upwards.
 */
class Type1Ime : InputMethodService(), KeyboardView.Listener {

	private val controller = KeyboardController()
	private var keyboardView: KeyboardView? = null
	private var selectAnchor = -1
	private var selectWidth = 0
	private var lastActionAt = 0L
	private var expectedSelStart = -1
	private var expectedSelEnd = -1
	private var allowNextSelection = false

	override fun onCreateInputView(): View {
		val view = KeyboardView(this, controller)
		view.listener = this
		view.setSpaceCursorEnabled(Prefs.spaceCursor(this))
		view.setBackspaceHold(Prefs.backspaceHold(this))
		view.setSwipeSensitivity(Prefs.swipeSensitivity(this))
		view.setUtilityRowHeight(Prefs.utilityRowHeight(this))
		view.setOnApplyWindowInsetsListener { _, insets ->
			view.setBottomInset(navigationBarHeight(insets))
			insets
		}
		keyboardView = view
		view.refresh()
		return view
	}

	override fun onStartInput(attribute: EditorInfo?, restarting: Boolean) {
		super.onStartInput(attribute, restarting)
		expectedSelStart = -1
		expectedSelEnd = -1
		allowNextSelection = false
		lastActionAt = 0L
		controller.onStartInput(attribute)
	}

	override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
		super.onStartInputView(info, restarting)
		// Some IME windows do not dispatch insets to the input view; query the
		// window directly as well.
		applyNavigationInset()
		keyboardView?.setSpaceCursorEnabled(Prefs.spaceCursor(this))
		keyboardView?.setBackspaceHold(Prefs.backspaceHold(this))
		keyboardView?.setSwipeSensitivity(Prefs.swipeSensitivity(this))
		keyboardView?.setUtilityRowHeight(Prefs.utilityRowHeight(this))
		selectAnchor = -1
		selectWidth = 0
		keyboardView?.refresh()
		syncContextFromEditor()
	}

	override fun onFinishInput() {
		super.onFinishInput()
		controller.reset()
		expectedSelStart = -1
		expectedSelEnd = -1
		allowNextSelection = false
		selectAnchor = -1
		selectWidth = 0
		keyboardView?.refresh()
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
		var cursorMoved = false
		// Keep replacement (delete + commit) atomic for the editor.
		ic.beginBatchEdit()
		try {
			for (action in actions) {
				when (action) {
					is KeyAction.Commit -> ic.commitText(action.text, 1)
					is KeyAction.DeleteWord -> deleteWord(ic, action.word)
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
					is KeyAction.DeleteSelection -> deleteSelection(ic)
					is KeyAction.Ime -> performEditorAction(ic, action.action)
				}
			}
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
			val before = ic.getTextBeforeCursor(MAX_SELECTION_CHARS, 0)
			val cursor = if (before != null && before.length < MAX_SELECTION_CHARS) {
				before.length
			} else {
				-1
			}
			expectedSelStart = cursor
			expectedSelEnd = cursor
			allowNextSelection = false
		}
		lastActionAt = SystemClock.uptimeMillis()
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

	/** Read the word around the caret and hand it to the core. */
	private fun syncContextFromEditor() {
		val ic = currentInputConnection
		if (ic == null) {
			controller.selectionChanged()
			keyboardView?.refresh()
			return
		}
		val before = ic.getTextBeforeCursor(MAX_CONTEXT_CHARS, 0)?.toString().orEmpty()
		val after = ic.getTextAfterCursor(MAX_CONTEXT_CHARS, 0)?.toString().orEmpty()
		val prefix = before.takeLastWhile { it.isWordChar() }
		val suffix = after.takeWhile { it.isWordChar() }
		if (prefix.length + suffix.length < 2) {
			controller.selectionChanged()
		} else {
			controller.syncContext(prefix, suffix)
		}
		keyboardView?.refresh()
	}

	private fun Char.isWordChar(): Boolean = isLetter() || this == '\'' || this == '’'

	private fun sendCursor(ic: InputConnection, delta: Int) {
		if (delta == 0) return
		val keyCode = if (delta < 0) KeyEvent.KEYCODE_DPAD_LEFT else KeyEvent.KEYCODE_DPAD_RIGHT
		repeat(abs(delta)) {
			ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, keyCode))
			ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, keyCode))
		}
	}

	private fun startSelection(ic: InputConnection) {
		val before = ic.getTextBeforeCursor(MAX_SELECTION_CHARS, 0)
		selectAnchor = if (before != null && before.length < MAX_SELECTION_CHARS) {
			before.length
		} else {
			-1
		}
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

	private fun deleteWord(ic: InputConnection, word: Boolean) {
		if (!word) {
			ic.deleteSurroundingText(1, 0)
			return
		}
		val before = ic.getTextBeforeCursor(64, 0)?.toString().orEmpty()
		var index = before.length
		while (index > 0 && before[index - 1].isWhitespace()) index--
		while (index > 0 && !before[index - 1].isWhitespace()) index--
		val count = before.length - index
		if (count > 0) ic.deleteSurroundingText(count, 0) else ic.deleteSurroundingText(1, 0)
	}

	private fun performEditorAction(ic: InputConnection, action: String) {		val id = when (action) {
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
		const val MAX_CONTEXT_CHARS = 64
		const val SELECTION_IGNORE_MS = 200L
	}
}
