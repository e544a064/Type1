package fr.type1.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import android.util.AttributeSet
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.widget.PopupWindow
import fr.type1.keyboard.core.KeyboardController
import fr.type1.keyboard.model.KeyAction
import fr.type1.keyboard.model.LetterKey
import fr.type1.keyboard.model.RowModel
import fr.type1.keyboard.model.SUGGESTION_PREFIX
import fr.type1.keyboard.model.UtilityKey
import kotlin.math.abs
import kotlin.math.min

/**
 * Three horizontal bands, driven entirely by [RowModel]:
 *
 * 1. suggestion strip (shift, suggestions, backspace)
 * 2. every letter on one line, no groups, row-based vertical offsets
 * 3. utility keys (page, space, comma, enter)
 */
@SuppressLint("ClickableViewAccessibility", "ViewConstructor")
class KeyboardView(
	context: Context,
	private val controller: KeyboardController,
	attrs: AttributeSet? = null,
) : View(context, attrs) {

	interface Listener {
		fun onActions(actions: List<KeyAction>)
		fun onCycleProfile()
		fun onEmoji(emoji: String)
		fun onOpenSettings()
		/** Grapheme-cluster room left and right of the caret for a spacebar drag. */
		fun onSpaceDragStart(): Pair<Int, Int>
	}

	var listener: Listener? = null

	private enum class Band { None, Suggestions, Letters, Utilities }

	private class AltEntry(
		val keyId: String,
		val display: String,
		val icon: String = "",
	)

	private val handler = Handler(Looper.getMainLooper())
	private val audio = context.getSystemService(Context.AUDIO_SERVICE) as AudioManager

	private var model: RowModel? = null

	private val suggestionHeight = dp(38f)
	private val letterHeight = dp(56f)
	private var utilityHeight = dp(48f)
	private val bandGap = dp(3f)
	private val padding = dp(3f)
	private val corner = dp(8f)

	private val suggestionTop = padding
	private val letterTop = padding + suggestionHeight + bandGap
	private val utilityTop = letterTop + letterHeight + bandGap
	private val contentHeight: Float get() = utilityTop + utilityHeight + padding

	/** Height of the system navigation bar overlapping the IME window. */
	private var bottomInset = 0f
	private val totalHeight: Float get() = contentHeight + bottomInset

	private val stripLeft = RectF()
	private val stripRight = RectF()
	private val suggestionRects = ArrayList<RectF>()
	private val letterRects = ArrayList<RectF>()
	private var letterTextSize = dp(16f)
	private val utilityRects = ArrayList<RectF>()
	private var altPopup: PopupWindow? = null
	private var altPopupView: AltView? = null
	private var altEntries: List<AltEntry> = emptyList()
	private var altSelected = -1
	private var altPopupLeft = 0f
	private var altCellWidth = 0f

	private var band = Band.None
	private var pressedLetter = -1
	private var pressedUtility = -1
	private var pressedSuggestion = -1
	private var pressedStrip = -1
	private var downUtility = -1
	private var downStrip = -1
	private var downBackspace = false
	private var longPressFired = false
	private var profileCount = 0
	private var emojiPicker: EmojiPickerPopup? = null
	private var spaceCursorEnabled = true
	private var spaceDragging = false
	private var spaceAnchorX = 0f
	private var spaceMoved = false
	private var spaceRoomLeft = 0
	private var spaceRoomRight = 0
	private var spaceSteps = 0
	private var spaceBudgetLoaded = false
	private var backspaceHold = BackspaceHold.Select
	private var selecting = false
	private var selectAnchorX = 0f
	private var selectSteps = 0
	private var lastX = 0f
	private var downX = 0f
	private var activePointerId = -1
	private var swipeStepDp = 20

	private val backgroundPaint = Paint().apply {
		color = context.getColor(R.color.keyboard_background)
	}
	private val keyPaint = Paint(Paint.ANTI_ALIAS_FLAG)
	private val labelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
		typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
		textAlign = Paint.Align.CENTER
	}

	private val colors = Colors(
		key = context.getColor(R.color.key_background),
		pressed = context.getColor(R.color.key_background_pressed),
		accent = context.getColor(R.color.accent),
		text = context.getColor(R.color.key_text),
		muted = context.getColor(R.color.key_text_muted),
		strip = context.getColor(R.color.suggestion_background),
	)

	private val iconDrawables: Map<String, Drawable> by lazy {
		val map = HashMap<String, Drawable>()
		fun add(name: String, res: Int) {
			context.getDrawable(res)?.let { map[name] = it }
		}
		add("shift", R.drawable.ic_shift)
		add("shift-locked", R.drawable.ic_shift_locked)
		add("backspace", R.drawable.ic_backspace)
		add("enter", R.drawable.ic_enter)
		add("arrow-right", R.drawable.ic_arrow_right)
		add("arrow-left", R.drawable.ic_arrow_left)
		add("check", R.drawable.ic_check)
		add("search", R.drawable.ic_search)
		add("send", R.drawable.ic_send)
		add("planet", R.drawable.ic_planet)
		map
	}

	private data class Colors(
		val key: Int,
		val pressed: Int,
		val accent: Int,
		val text: Int,
		val muted: Int,
		val strip: Int,
	)

	private val longPressRunnable = Runnable {
		when (band) {
			Band.Letters -> {
				if (showAltPopup(pressedLetter)) {
					longPressFired = true
					performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
				}
			}
			Band.Utilities -> {
				val key = model?.keys?.getOrNull(downUtility) ?: return@Runnable
				when {
					key.isBackspace() -> handleBackspaceHold()
					key.kind == "page" && showPageAltPopup(downUtility) -> {
						longPressFired = true
						performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
					}
					key.id == "comma" && showCommaPopup(downUtility) -> {
						longPressFired = true
						performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
					}
					key.alts.isNotEmpty() && showUtilityAltPopup(downUtility) -> {
						longPressFired = true
						performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
					}
				}
			}
			Band.Suggestions -> {
				val key = model?.strip?.getOrNull(downStrip) ?: return@Runnable
				if (key.isBackspace()) handleBackspaceHold()
			}
			else -> Unit
		}
	}

	private val repeatRunnable = object : Runnable {
		override fun run() {
			emit(controller.press("backspace"))
			handler.postDelayed(this, REPEAT_MS)
		}
	}

	init {
		setBackgroundColor(context.getColor(R.color.keyboard_background))
	}

	/** Re-pull the three bands from the core. */
	fun refresh() {
		val next = controller.render()
		model = next
		dismissAltPopup()
		layoutBands(next)
		invalidate()
	}

	/** Reserve space for the system navigation bar so it never covers keys. */
	fun setBottomInset(pixels: Float) {
		if (pixels == bottomInset) return
		bottomInset = pixels
		requestLayout()
		invalidate()
	}

	/** Enable or disable moving the caret by dragging on the spacebar. */
	fun setSpaceCursorEnabled(enabled: Boolean) {
		spaceCursorEnabled = enabled
		if (!enabled) spaceDragging = false
	}

	/** Choose what a long press on the backspace key does. */
	fun setBackspaceHold(mode: BackspaceHold) {
		backspaceHold = mode
	}

	/** Number of configured profiles; the comma popup cycles when there are 2+. */
	fun setProfileCount(count: Int) {
		profileCount = count
	}

	private fun openEmojiPicker() {
		emojiPicker?.dismiss()
		val picker = EmojiPickerPopup(
			context,
			recentEmojis = { Prefs.recentEmojis(context) },
			onPick = { emoji ->
				listener?.onEmoji(emoji)
				playClick()
				performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
			},
			onDismissed = { if (emojiPicker === it) emojiPicker = null },
		)
		if (picker.show(this)) emojiPicker = picker
	}

	/** Hide the emoji panel, for example when the IME window goes away. */
	fun dismissEmojiPicker() {
		emojiPicker?.dismiss()
		emojiPicker = null
	}

	private fun UtilityKey?.isBackspace(): Boolean = this?.kind == "backspace"

	/** dp of horizontal travel per caret/selection step (8-40). */
	fun setSwipeSensitivity(dpPerStep: Int) {
		swipeStepDp = dpPerStep.coerceIn(8, 40)
	}

	/** Height in dp of the bottom row holding the spacebar (36-72). */
	fun setUtilityRowHeight(heightDp: Int) {
		val height = dp(heightDp.coerceIn(36, 72).toFloat())
		if (height == utilityHeight) return
		utilityHeight = height
		requestLayout()
		invalidate()
	}

	override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
		setMeasuredDimension(MeasureSpec.getSize(widthMeasureSpec), totalHeight.toInt() + 1)
	}

	override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
		super.onSizeChanged(w, h, oldw, oldh)
		model?.let { layoutBands(it) }
	}

	private fun layoutBands(row: RowModel) {
		if (width == 0) return

		stripLeft.set(padding, suggestionTop, padding + suggestionHeight, suggestionTop + suggestionHeight)
		stripRight.set(
			width - padding - suggestionHeight,
			suggestionTop,
			width - padding,
			suggestionTop + suggestionHeight,
		)

		suggestionRects.clear()
		val suggestionsLeft = stripLeft.right + dp(4f)
		val suggestionsRight = stripRight.left - dp(4f)
		if (row.suggestions.isNotEmpty() && suggestionsRight > suggestionsLeft) {
			val each = (suggestionsRight - suggestionsLeft) / row.suggestions.size
			var x = suggestionsLeft
			for (i in row.suggestions.indices) {
				suggestionRects += RectF(x, suggestionTop, x + each, suggestionTop + suggestionHeight)
				x += each
			}
		}

		layoutLetters(row.letters)
		layoutUtilities(row.keys)
	}

	private fun layoutLetters(letters: List<LetterKey>) {
		letterRects.clear()
		if (letters.isEmpty()) return
		val available = width - padding * 2
		if (available <= 0f) return
		val each = available / letters.size
		var x = padding
		for (i in letters.indices) {
			letterRects += RectF(x, letterTop, x + each, letterTop + letterHeight)
			x += each
		}

		// One size for the whole row, measured on the uppercase labels so it
		// does not jump when shift toggles.
		var size = dp(17f)
		val minimum = dp(9f)
		labelPaint.textSize = size
		while (size > minimum && letters.any { labelPaint.measureText(it.label) > each - dp(1f) }) {
			size -= 0.5f
			labelPaint.textSize = size
		}
		letterTextSize = size
	}

	private fun layoutUtilities(keys: List<UtilityKey>) {
		utilityRects.clear()
		if (keys.isEmpty()) return
		val keyGap = dp(4f)
		val available = width - padding * 2 - keyGap * (keys.size - 1)
		val total = keys.sumOf { it.weight.toDouble() }.toFloat()
		if (available <= 0f || total <= 0f) return
		val unit = available / total
		var x = padding
		for (key in keys) {
			val keyWidth = unit * key.weight
			utilityRects += RectF(x, utilityTop, x + keyWidth, utilityTop + utilityHeight)
			x += keyWidth + keyGap
		}
	}

	override fun onDraw(canvas: Canvas) {
		canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), backgroundPaint)
		val row = model ?: return
		drawSuggestionBand(canvas, row)
		drawUtilities(canvas, row)
		drawLetters(canvas, row)
	}

	private fun drawSuggestionBand(canvas: Canvas, row: RowModel) {
		keyPaint.color = colors.strip
		canvas.drawRoundRect(
			padding,
			suggestionTop,
			width - padding,
			suggestionTop + suggestionHeight,
			corner,
			corner,
			keyPaint,
		)

		labelPaint.color = colors.muted
		labelPaint.textSize = suggestionHeight * 0.40f
		drawStripKey(canvas, stripLeft, row.strip.getOrNull(0), 0, row.shift)
		drawStripKey(canvas, stripRight, row.strip.getOrNull(1), 1, row.shift)

		for (i in row.suggestions.indices) {
			val rect = suggestionRects.getOrNull(i) ?: continue
			val pressed = pressedSuggestion == i
			if (pressed) {
				keyPaint.color = colors.accent
				canvas.drawRoundRect(rect, corner, corner, keyPaint)
			}
			labelPaint.color = if (pressed) Color.WHITE else colors.text
			labelPaint.textSize = fitText(
				row.suggestions[i],
				rect.width() - dp(8f),
				dp(16f),
				dp(10f),
			)
			canvas.drawText(row.suggestions[i], rect.centerX(), baseline(rect), labelPaint)
		}
	}

	/** Shift on the left, backspace on the right: real keys inside the strip. */
	private fun drawStripKey(canvas: Canvas, rect: RectF, key: UtilityKey?, index: Int, shift: Int) {
		if (key == null) return
		val pressed = pressedStrip == index
		val highlighted = pressed || key.active
		keyPaint.color = when {
			pressed -> colors.pressed
			key.active -> colors.accent
			else -> colors.key
		}
		canvas.drawRoundRect(rect, corner, corner, keyPaint)
		// Shift once highlights the arrow; caps lock adds a bar under it.
		val icon = if (key.icon == "shift" && shift == SHIFT_LOCKED) "shift-locked" else key.icon
		val tint = if (highlighted) Color.WHITE else colors.muted
		if (icon.isNotEmpty()) {
			drawIcon(canvas, rect, icon, tint, 20f)
			return
		}
		labelPaint.color = tint
		labelPaint.textSize = fitText(
			key.label,
			rect.width() - dp(8f),
			suggestionHeight * 0.44f,
			dp(10f),
		)
		canvas.drawText(key.label, rect.centerX(), baseline(rect), labelPaint)
	}

	private fun drawLetters(canvas: Canvas, row: RowModel) {
		for (i in row.letters.indices) {
			val rect = letterRects.getOrNull(i) ?: continue
			val letter = row.letters[i]
			val dy = rowOffset(letter)
			val pressed = pressedLetter == i
			if (pressed) {
				keyPaint.color = colors.accent
				val half = dp(19f)
				val center = rect.centerY() + dy
				canvas.drawRoundRect(
					RectF(rect.left + dp(1f), center - half, rect.right - dp(1f), center + half),
					dp(6f),
					dp(6f),
					keyPaint,
				)
			}
			labelPaint.color = if (pressed) Color.WHITE else colors.text
			labelPaint.textSize = letterTextSize
			val label = displayLabel(letter, row.caps)
			canvas.drawText(label, rect.centerX(), baseline(rect) + dy, labelPaint)
		}
	}

	/** Uppercase while shift (or auto-capitals) is active, lowercase otherwise. */
	private fun displayLabel(letter: LetterKey, caps: Boolean): String {
		if (!letter.letter) return letter.label
		return if (caps) letter.label.uppercase() else letter.label.lowercase()
	}

	/** Top-row letters sit higher, bottom-row letters lower. */
	private fun rowOffset(letter: LetterKey): Float {
		return when (letter.row) {
			0 -> -dp(7f)
			2 -> dp(7f)
			else -> 0f
		}
	}

	private fun drawUtilities(canvas: Canvas, row: RowModel) {
		for (i in row.keys.indices) {
			val rect = utilityRects.getOrNull(i) ?: continue
			val key = row.keys[i]
			val pressed = pressedUtility == i
			val highlighted = pressed || key.kind == "enter" || key.active
			keyPaint.color = when {
				pressed -> colors.pressed
				highlighted -> colors.accent
				else -> colors.key
			}
			canvas.drawRoundRect(rect, corner, corner, keyPaint)

			if (key.icon.isNotEmpty()) {
				drawIcon(canvas, rect, key.icon, if (highlighted) Color.WHITE else colors.muted, 22f)
				continue
			}

			// The page key has ten digit alternates and the period key has a
			// full symbol set; showing all of them as a hint would be noise.
			if (key.alts.isNotEmpty() && key.kind != "page") {
				val hint = if (key.alts.size <= 3) {
					key.alts.joinToString(" ")
				} else {
					"…"
				}
				labelPaint.color = colors.muted
				labelPaint.textAlign = Paint.Align.RIGHT
				labelPaint.textSize = fitText(hint, rect.width() - dp(4f), dp(10f), dp(7f))
				canvas.drawText(hint, rect.right - dp(3f), rect.top + labelPaint.textSize, labelPaint)
				labelPaint.textAlign = Paint.Align.CENTER
			}

			labelPaint.color = when (key.kind) {
				"space", "page" -> colors.muted
				else -> colors.text
			}
			labelPaint.textSize = fitText(key.label, rect.width() - dp(6f), dp(16f), dp(9f))
			canvas.drawText(key.label, rect.centerX(), baseline(rect), labelPaint)
		}
	}

	private fun drawIcon(canvas: Canvas, rect: RectF, name: String, color: Int, sizeDp: Float) {
		val drawable = iconDrawables[name] ?: return
		val size = dp(sizeDp)
		val left = (rect.centerX() - size / 2f).toInt()
		val top = (rect.centerY() - size / 2f).toInt()
		drawable.setBounds(left, top, left + size.toInt(), top + size.toInt())
		drawable.setTint(color)
		drawable.draw(canvas)
	}

	/** Floating bubble with the accents of the pressed key and its neighbours. */
	private fun showAltPopup(index: Int): Boolean {
		val letters = model?.letters ?: return false
		val letter = letters.getOrNull(index) ?: return false
		val rect = letterRects.getOrNull(index) ?: return false

		val entries = ArrayList<AltEntry>()
		letters.getOrNull(index - 1)?.alts?.forEach { entries += AltEntry("alt:$it", it) }
		val baseIndex = entries.size
		val baseLabel = if (model?.caps == true) letter.label else letter.label.lowercase()
		entries += AltEntry("alt:${letter.id}", baseLabel)
		letter.alts.forEach { entries += AltEntry("alt:$it", it) }
		letters.getOrNull(index + 1)?.alts?.forEach { entries += AltEntry("alt:$it", it) }
		if (entries.size <= 1) return false
		return showAltEntries(entries, rect, baseIndex, letterTop)
	}

	/** Long-press popup for a bottom-row key with symbols (the period). */
	private fun showUtilityAltPopup(index: Int): Boolean {
		val key = model?.keys?.getOrNull(index) ?: return false
		val rect = utilityRects.getOrNull(index) ?: return false
		if (key.alts.isEmpty()) return false

		val entries = ArrayList<AltEntry>()
		// Keep the key's own symbol between its alternates, so the finger
		// starts on the symbol that was pressed.
		entries += AltEntry("sym:${key.alts.first()}", key.alts.first())
		val baseIndex = entries.size
		entries += AltEntry("sym:${key.label}", key.label)
		for (alt in key.alts.drop(1)) {
			entries += AltEntry("sym:$alt", alt)
		}
		return showAltEntries(entries, rect, baseIndex, utilityTop)
	}

	/** Long-press popup on the page key: type one digit without switching page. */
	private fun showPageAltPopup(index: Int): Boolean {
		val key = model?.keys?.getOrNull(index) ?: return false
		val rect = utilityRects.getOrNull(index) ?: return false
		if (key.alts.isEmpty()) return false
		val entries = key.alts.map { AltEntry("sym:$it", it) }
		// Anchor the first cell over the page key and start unselected, so a
		// long press alone never types a digit.
		return showAltEntries(entries, rect, 0, utilityTop, initialSelected = -1)
	}

	/**
	 * Long-press on the comma: emoji picker, plus the profile switcher when
	 * there is another profile to cycle to.
	 */
	private fun showCommaPopup(index: Int): Boolean {
		val rect = utilityRects.getOrNull(index) ?: return false
		val entries = ArrayList<AltEntry>()
		if (profileCount >= 2) {
			entries += AltEntry(PROFILE_NEXT, "", "planet")
		}
		entries += AltEntry(EMOJI_PICKER, "🙂")
		entries += AltEntry(SETTINGS_OPEN, "\u2699")
		return showAltEntries(
			entries,
			rect,
			0,
			utilityTop,
			initialSelected = 0,
			cellDp = PLANET_CELL_DP,
		)
	}

	private fun showAltEntries(
		entries: List<AltEntry>,
		rect: RectF,
		baseIndex: Int,
		bandTop: Float,
		initialSelected: Int = baseIndex,
		cellDp: Float = ALT_CELL_DP,
	): Boolean {
		dismissAltPopup()

		val count = entries.size
		val desiredWidth = count * dp(cellDp) + dp(8f)
		val bubbleWidth = min(desiredWidth, width - dp(8f))
		val bubbleHeight = dp(ALT_HEIGHT_DP)
		val cellWidth = bubbleWidth / count

		val popupView = AltView(context, entries)
		val popup = PopupWindow(popupView, bubbleWidth.toInt(), bubbleHeight.toInt(), false)
		popup.isTouchable = false
		popup.isFocusable = false
		popup.isOutsideTouchable = false
		popup.setBackgroundDrawable(null)
		popup.elevation = dp(8f)

		val location = IntArray(2)
		getLocationInWindow(location)
		// Center the pressed key's cell over the key, not the whole bubble, so
		// the finger starts on its own character.
		val baseCenter = (baseIndex + 0.5f) * cellWidth
		val maxLeft = (width - bubbleWidth).toInt()
		val leftInView = (
			(rect.centerX() - baseCenter).toInt()
			. coerceIn(0, maxOf(0, maxLeft))
		)
		val x = location[0] + leftInView
		val y = (location[1] + bandTop - bubbleHeight - dp(6f)).toInt()

		try {
			popup.showAtLocation(this, Gravity.NO_GRAVITY, x, y)
		} catch (_: Exception) {
			return false
		}

		altPopup = popup
		altPopupView = popupView
		altEntries = entries
		altSelected = initialSelected
		popupView.selected = initialSelected
		altCellWidth = cellWidth
		altPopupLeft = leftInView.toFloat()
		return true
	}

	private fun updateAltSelection(x: Float) {
		if (altPopup == null || altEntries.isEmpty()) return
		// The page-key popup starts unselected; ignore hand jitter until the
		// finger really slides.
		if (altSelected < 0 && abs(x - downX) < dp(ALT_SELECT_SLOP_DP)) return
		val index = (
			((x - altPopupLeft) / altCellWidth).toInt()
			. coerceIn(0, altEntries.size - 1)
		)
		if (index != altSelected) {
			altSelected = index
			altPopupView?.selected = index
			performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
		}
	}

	private fun dismissAltPopup() {
		altPopup?.dismiss()
		altPopup = null
		altPopupView = null
		altEntries = emptyList()
		altSelected = -1
	}

	private inner class AltView(context: Context, private val entries: List<AltEntry>) : View(context) {
		var selected = -1
			set(value) {
				field = value
				invalidate()
			}

		private val bubblePaint = Paint(Paint.ANTI_ALIAS_FLAG)
		private val popupTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
			typeface = Typeface.create("sans-serif-medium", Typeface.NORMAL)
			textAlign = Paint.Align.CENTER
		}

		override fun onDraw(canvas: Canvas) {
			bubblePaint.color = colors.key
			canvas.drawRoundRect(0f, 0f, width.toFloat(), height.toFloat(), corner, corner, bubblePaint)

			val cell = width.toFloat() / entries.size
			for (i in entries.indices) {
				val left = i * cell
				if (i == selected) {
					bubblePaint.color = colors.accent
					canvas.drawRoundRect(
						left + dp(2f),
						dp(2f),
						left + cell - dp(2f),
						height - dp(2f),
						corner,
						corner,
						bubblePaint,
					)
				}
				val entry = entries[i]
				popupTextPaint.color = if (i == selected) Color.WHITE else colors.text
				if (entry.icon.isNotEmpty()) {
					val iconSize = min(cell, height.toFloat()) * 0.56f
					val iconLeft = left + (cell - iconSize) / 2f
					val iconTop = (height - iconSize) / 2f
					iconDrawables[entry.icon]?.let { drawable ->
						drawable.setBounds(
							iconLeft.toInt(),
							iconTop.toInt(),
							(iconLeft + iconSize).toInt(),
							(iconTop + iconSize).toInt(),
						)
						drawable.setTint(popupTextPaint.color)
						drawable.draw(canvas)
					}
				} else {
					popupTextPaint.textSize = fitPopupText(entry.display, cell - dp(4f))
					val baseline = height / 2f - (popupTextPaint.ascent() + popupTextPaint.descent()) / 2f
					canvas.drawText(entry.display, left + cell / 2f, baseline, popupTextPaint)
				}
			}
		}

		private fun fitPopupText(text: String, maxWidth: Float): Float {
			var size = height * 0.46f
			popupTextPaint.textSize = size
			while (size > dp(9f) && popupTextPaint.measureText(text) > maxWidth) {
				size -= 0.5f
				popupTextPaint.textSize = size
			}
			return size
		}
	}

	private fun baseline(rect: RectF): Float {
		return rect.centerY() - (labelPaint.ascent() + labelPaint.descent()) / 2f
	}

	private fun fitText(text: String, maxWidth: Float, startSize: Float, minSize: Float): Float {
		var size = startSize
		labelPaint.textSize = size
		while (size > minSize && labelPaint.measureText(text) > maxWidth) {
			size -= 0.5f
			labelPaint.textSize = size
		}
		return size
	}

	private fun startPress(x: Float, y: Float) {
		longPressFired = false
		lastX = x
		downX = x
		band = bandAt(y)
		when (band) {
			Band.Suggestions -> {
				pressedStrip = when {
					stripLeft.contains(x, y) -> 0
					stripRight.contains(x, y) -> 1
					else -> -1
				}
				downStrip = pressedStrip
				downBackspace = model?.strip?.getOrNull(pressedStrip).isBackspace()
				if (pressedStrip >= 0) {
					performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
					scheduleLongPress()
				} else {
					pressedSuggestion = hit(suggestionRects, x)
				}
				invalidate()
			}
			Band.Letters -> {
				pressedLetter = hitLetter(x)
				if (pressedLetter >= 0) {
					performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
					scheduleLongPress()
				}
				invalidate()
			}
			Band.Utilities -> {
				downUtility = hit(utilityRects, x)
				pressedUtility = downUtility
				downBackspace = model?.keys?.getOrNull(downUtility).isBackspace()
				if (downUtility >= 0) {
					performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
					scheduleLongPress()
					spaceDragging = (
						spaceCursorEnabled
						&& model?.keys?.getOrNull(downUtility)?.kind == "space"
					)
					if (spaceDragging) {
						spaceAnchorX = x
						spaceMoved = false
						spaceSteps = 0
						spaceRoomLeft = 0
						spaceRoomRight = 0
						spaceBudgetLoaded = false
					}
				}
				invalidate()
			}
			Band.None -> Unit
		}
	}

	private fun finishPress() {
		cancelTimers()
		if (selecting) {
			selecting = false
			listener?.onActions(controller.press("delete_selection"))
			refresh()
			resetTouch()
			return
		}
		if (altPopup != null) {
			val entry = altEntries.getOrNull(altSelected)
			// A page-key popup starts unselected: releasing without sliding
			// keeps the normal page switch instead of typing a digit.
			val switchPage = (
				entry == null
				&& band == Band.Utilities
				&& model?.keys?.getOrNull(downUtility)?.kind == "page"
			)
			val cycleProfile = entry?.keyId == PROFILE_NEXT
			val openEmoji = entry?.keyId == EMOJI_PICKER
			val openSettings = entry?.keyId == SETTINGS_OPEN
			dismissAltPopup()
			when {
				cycleProfile -> listener?.onCycleProfile()
				openEmoji -> openEmojiPicker()
				openSettings -> listener?.onOpenSettings()
				entry != null -> emit(controller.press(entry.keyId))
				switchPage -> emit(controller.press("page"))
			}
			resetTouch()
			return
		}

		when (band) {
			Band.Suggestions -> when {
				pressedStrip >= 0 && !longPressFired -> model?.strip?.getOrNull(pressedStrip)?.let { emit(controller.press(it.id)) }
				pressedSuggestion >= 0 -> emit(controller.press("$SUGGESTION_PREFIX$pressedSuggestion"))
			}
			Band.Letters -> if (!longPressFired && pressedLetter >= 0) {
				model?.letters?.getOrNull(pressedLetter)?.let { emit(controller.press(it.id)) }
			}
			Band.Utilities -> {
				if (spaceDragging) {
					if (!spaceMoved) {
						model?.keys?.getOrNull(pressedUtility)?.let { emit(controller.press(it.id)) }
					}
				} else if (!longPressFired && pressedUtility >= 0 && pressedUtility == downUtility) {
					model?.keys?.getOrNull(pressedUtility)?.let { emit(controller.press(it.id)) }
				}
			}
			Band.None -> Unit
		}
		resetTouch()
	}

	override fun onTouchEvent(event: MotionEvent): Boolean {
		when (event.actionMasked) {
			MotionEvent.ACTION_DOWN -> {
				cancelTimers()
				resetTouch()
				activePointerId = event.getPointerId(0)
				startPress(event.x, event.y)
				return true
			}

			MotionEvent.ACTION_POINTER_DOWN -> {
				val index = event.actionIndex
				// Commit the held key first: keys must reach the editor in the
				// order they went down, even when several fingers overlap.
				finishPress()
				activePointerId = event.getPointerId(index)
				startPress(event.getX(index), event.getY(index))
				return true
			}

			MotionEvent.ACTION_MOVE -> {
				val index = event.findPointerIndex(activePointerId)
				if (index < 0) {
					resetTouch()
					return true
				}
				val x = event.getX(index)
				val y = event.getY(index)
				lastX = x
				// A clear leftward swipe on backspace means the user wants
				// selection mode; start it now instead of waiting for the
				// long-press timeout.
				if (
					!selecting
					&& downBackspace
					&& backspaceHold == BackspaceHold.Select
					&& x <= downX - dp(swipeStepDp.toFloat())
				) {
					cancelTimers()
					handleBackspaceHold()
					dragSelection(x)
					return true
				}
				if (selecting) {
					dragSelection(x)
					return true
				}
				if (altPopup != null) {
					updateAltSelection(x)
					return true
				}
				when (band) {
					Band.Letters -> {
						val letter = hitLetter(x)
						if (letter != pressedLetter) {
							pressedLetter = letter
							invalidate()
						}
					}
					Band.Utilities -> {
						if (spaceDragging) {
							dragSpaceCursor(x)
						} else {
							val utility = hit(utilityRects, x)
							if (utility != pressedUtility) {
								pressedUtility = utility
								invalidate()
							}
						}
					}
					Band.Suggestions -> {
						if (pressedStrip >= 0) {
							val next = when {
								stripLeft.contains(x, y) -> 0
								stripRight.contains(x, y) -> 1
								else -> -1
							}
							if (next != pressedStrip) {
								pressedStrip = next
								invalidate()
							}
						} else {
							val suggestion = hit(suggestionRects, x)
							if (suggestion != pressedSuggestion) {
								pressedSuggestion = suggestion
								invalidate()
							}
						}
					}
					Band.None -> Unit
				}
				return true
			}

			MotionEvent.ACTION_POINTER_UP -> {
				val index = event.actionIndex
				if (event.getPointerId(index) == activePointerId) {
					finishPress()
				}
				return true
			}

			MotionEvent.ACTION_UP -> {
				finishPress()
				return true
			}

			MotionEvent.ACTION_CANCEL -> {
				cancelTimers()
				resetTouch()
				return true
			}
		}
		return true
	}

	private fun bandAt(y: Float): Band {
		return when {
			y < letterTop - bandGap / 2f -> Band.Suggestions
			y < utilityTop - bandGap / 2f -> Band.Letters
			y <= height.toFloat() -> Band.Utilities
			else -> Band.None
		}
	}

	private fun hit(rects: List<RectF>, x: Float): Int {
		for (i in rects.indices) {
			if (x >= rects[i].left && x <= rects[i].right) return i
		}
		return -1
	}

	/** Nearest-letter lookup, so the gaps between groups stay typable. */
	private fun hitLetter(x: Float): Int {
		if (letterRects.isEmpty()) return -1
		val direct = hit(letterRects, x)
		if (direct >= 0) return direct
		var best = -1
		var bestDistance = Float.MAX_VALUE
		for (i in letterRects.indices) {
			val rect = letterRects[i]
			val distance = min(abs(x - rect.left), abs(x - rect.right))
			if (distance < bestDistance) {
				bestDistance = distance
				best = i
			}
		}
		return best
	}

	private fun scheduleLongPress() {
		handler.removeCallbacks(longPressRunnable)
		handler.postDelayed(longPressRunnable, LONG_PRESS_MS)
	}

	private fun emit(actions: List<KeyAction>) {
		if (actions.isNotEmpty()) {
			playClick()
			listener?.onActions(actions)
		}
		refresh()
	}

	/** One caret step per configured number of dp of horizontal travel. */
	private fun dragSpaceCursor(x: Float) {
		val step = dp(swipeStepDp.toFloat())
		var delta = x - spaceAnchorX
		if (!spaceBudgetLoaded && abs(delta) >= step) {
			// Read the caret room on the first actual drag, not on every tap.
			val room = listener?.onSpaceDragStart()
			spaceRoomLeft = room?.first ?: 0
			spaceRoomRight = room?.second ?: 0
			spaceBudgetLoaded = true
		}
		while (abs(delta) >= step) {
			val direction = if (delta > 0f) 1 else -1
			val blocked = (
				(direction > 0 && spaceSteps >= spaceRoomRight)
				|| (direction < 0 && spaceSteps <= -spaceRoomLeft)
			)
			if (blocked) {
				// There is no room for another caret step. Follow the finger so
				// reversing direction responds immediately, and suppress the
				// space tap because this was a drag gesture.
				spaceMoved = true
				spaceAnchorX = x
				break
			}
			spaceAnchorX += direction * step
			spaceSteps += direction
			listener?.onActions(
				controller.press(if (direction > 0) "cursor_right" else "cursor_left")
			)
			if (!spaceMoved) {
				spaceMoved = true
				// The caret moved: suggestions for the old word are stale.
				refresh()
			}
			delta = x - spaceAnchorX
		}
	}

	private fun startRepeat() {
		handler.post(repeatRunnable)
	}

	private fun handleBackspaceHold() {
		when (backspaceHold) {
			BackspaceHold.None -> Unit
			BackspaceHold.Repeat -> {
				longPressFired = true
				startRepeat()
			}
			BackspaceHold.Select -> {
				longPressFired = true
				selecting = true
				// Count any movement already made during the hold: using only
				// the position at long-press time made the first character
				// need that extra travel before it appeared.
				selectAnchorX = maxOf(downX, lastX)
				selectSteps = 0
				listener?.onActions(controller.press("start_selection"))
				performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
			}
		}
	}

	/** One selection character per configured number of dp between finger and anchor. */
	private fun dragSelection(x: Float) {
		val step = dp(swipeStepDp.toFloat())
		val target = maxOf(0, ((selectAnchorX - x) / step).toInt())
		while (selectSteps < target) {
			selectSteps++
			listener?.onActions(controller.press("select_left"))
		}
		while (selectSteps > target) {
			selectSteps--
			listener?.onActions(controller.press("select_right"))
		}
	}

	private fun cancelTimers() {
		handler.removeCallbacks(longPressRunnable)
		handler.removeCallbacks(repeatRunnable)
	}

	private fun resetTouch() {
		band = Band.None
		activePointerId = -1
		pressedLetter = -1
		pressedUtility = -1
		pressedSuggestion = -1
		pressedStrip = -1
		downUtility = -1
		downStrip = -1
		downBackspace = false
		dismissAltPopup()
		longPressFired = false
		spaceDragging = false
		spaceMoved = false
		spaceAnchorX = 0f
		spaceRoomLeft = 0
		spaceRoomRight = 0
		spaceSteps = 0
		spaceBudgetLoaded = false
		selecting = false
		selectSteps = 0
		selectAnchorX = 0f
		invalidate()
	}

	private fun playClick() {
		try {
			audio.playSoundEffect(AudioManager.FX_KEYPRESS_STANDARD, 0.6f)
		} catch (_: Exception) {
			// Sound is best effort.
		}
	}

	override fun onDetachedFromWindow() {
		handler.removeCallbacksAndMessages(null)
		dismissEmojiPicker()
		dismissAltPopup()
		super.onDetachedFromWindow()
	}

	private fun dp(value: Float): Float = value * resources.displayMetrics.density

	private companion object {
		// Long press tells a tap apart from a hold. A deliberate leftward
		// swipe on backspace starts selection before this timer fires.
		const val LONG_PRESS_MS = 320L
		const val REPEAT_MS = 55L
		const val ALT_CELL_DP = 40f
		const val ALT_HEIGHT_DP = 44f
		const val ALT_SELECT_SLOP_DP = 6f
		const val PLANET_CELL_DP = 64f
		const val PROFILE_NEXT = "profile:next"
		const val EMOJI_PICKER = "emoji:picker"
		const val SETTINGS_OPEN = "settings:open"
		// ShiftState::Locked in the core.
		const val SHIFT_LOCKED = 2
	}
}
