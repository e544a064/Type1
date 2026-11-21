package fr.type1.keyboard

import android.inputmethodservice.InputMethodService
import android.os.Build
import android.provider.Settings
import android.view.View
import android.view.WindowInsets
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import fr.type1.keyboard.core.KeyboardController
import fr.type1.keyboard.model.KeyAction

/**
 * The IME itself. All keyboard logic lives in the Rust core; this service only
 * feeds editor information down and applies the resulting actions upwards.
 */
class Type1Ime : InputMethodService(), KeyboardView.Listener {

	private val controller = KeyboardController()
	private var keyboardView: KeyboardView? = null

	override fun onCreateInputView(): View {
		val view = KeyboardView(this, controller)
		view.listener = this
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
		controller.onStartInput(attribute)
	}

	override fun onStartInputView(info: EditorInfo?, restarting: Boolean) {
		super.onStartInputView(info, restarting)
		// Some IME windows do not dispatch insets to the input view; query the
		// window directly as well.
		applyNavigationInset()
		keyboardView?.refresh()
	}

	override fun onFinishInput() {
		super.onFinishInput()
		controller.reset()
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
					is KeyAction.Ime -> performEditorAction(ic, action.action)
				}
			}
		} finally {
			ic.endBatchEdit()
		}
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
}
