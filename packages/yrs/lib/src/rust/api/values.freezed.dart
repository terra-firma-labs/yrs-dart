// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'values.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;

/// @nodoc
mixin _$YInValue {
  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is YInValue);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'YInValue()';
  }
}

/// @nodoc
class $YInValueCopyWith<$Res> {
  $YInValueCopyWith(YInValue _, $Res Function(YInValue) __);
}

/// Adds pattern-matching-related methods to [YInValue].
extension YInValuePatterns on YInValue {
  /// A variant of `map` that fallback to returning `orElse`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(YInValue_String value)? string,
    TResult Function(YInValue_Int value)? int,
    TResult Function(YInValue_Double value)? double,
    TResult Function(YInValue_Bool value)? bool,
    TResult Function(YInValue_Null value)? null_,
    TResult Function(YInValue_Bytes value)? bytes,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case YInValue_String() when string != null:
        return string(_that);
      case YInValue_Int() when int != null:
        return int(_that);
      case YInValue_Double() when double != null:
        return double(_that);
      case YInValue_Bool() when bool != null:
        return bool(_that);
      case YInValue_Null() when null_ != null:
        return null_(_that);
      case YInValue_Bytes() when bytes != null:
        return bytes(_that);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// Callbacks receives the raw object, upcasted.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case final Subclass2 value:
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(YInValue_String value) string,
    required TResult Function(YInValue_Int value) int,
    required TResult Function(YInValue_Double value) double,
    required TResult Function(YInValue_Bool value) bool,
    required TResult Function(YInValue_Null value) null_,
    required TResult Function(YInValue_Bytes value) bytes,
  }) {
    final _that = this;
    switch (_that) {
      case YInValue_String():
        return string(_that);
      case YInValue_Int():
        return int(_that);
      case YInValue_Double():
        return double(_that);
      case YInValue_Bool():
        return bool(_that);
      case YInValue_Null():
        return null_(_that);
      case YInValue_Bytes():
        return bytes(_that);
    }
  }

  /// A variant of `map` that fallback to returning `null`.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case final Subclass value:
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(YInValue_String value)? string,
    TResult? Function(YInValue_Int value)? int,
    TResult? Function(YInValue_Double value)? double,
    TResult? Function(YInValue_Bool value)? bool,
    TResult? Function(YInValue_Null value)? null_,
    TResult? Function(YInValue_Bytes value)? bytes,
  }) {
    final _that = this;
    switch (_that) {
      case YInValue_String() when string != null:
        return string(_that);
      case YInValue_Int() when int != null:
        return int(_that);
      case YInValue_Double() when double != null:
        return double(_that);
      case YInValue_Bool() when bool != null:
        return bool(_that);
      case YInValue_Null() when null_ != null:
        return null_(_that);
      case YInValue_Bytes() when bytes != null:
        return bytes(_that);
      case _:
        return null;
    }
  }

  /// A variant of `when` that fallback to an `orElse` callback.
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return orElse();
  /// }
  /// ```

  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? string,
    TResult Function(PlatformInt64 field0)? int,
    TResult Function(double field0)? double,
    TResult Function(bool field0)? bool,
    TResult Function()? null_,
    TResult Function(Uint8List field0)? bytes,
    required TResult orElse(),
  }) {
    final _that = this;
    switch (_that) {
      case YInValue_String() when string != null:
        return string(_that.field0);
      case YInValue_Int() when int != null:
        return int(_that.field0);
      case YInValue_Double() when double != null:
        return double(_that.field0);
      case YInValue_Bool() when bool != null:
        return bool(_that.field0);
      case YInValue_Null() when null_ != null:
        return null_();
      case YInValue_Bytes() when bytes != null:
        return bytes(_that.field0);
      case _:
        return orElse();
    }
  }

  /// A `switch`-like method, using callbacks.
  ///
  /// As opposed to `map`, this offers destructuring.
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case Subclass2(:final field2):
  ///     return ...;
  /// }
  /// ```

  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) string,
    required TResult Function(PlatformInt64 field0) int,
    required TResult Function(double field0) double,
    required TResult Function(bool field0) bool,
    required TResult Function() null_,
    required TResult Function(Uint8List field0) bytes,
  }) {
    final _that = this;
    switch (_that) {
      case YInValue_String():
        return string(_that.field0);
      case YInValue_Int():
        return int(_that.field0);
      case YInValue_Double():
        return double(_that.field0);
      case YInValue_Bool():
        return bool(_that.field0);
      case YInValue_Null():
        return null_();
      case YInValue_Bytes():
        return bytes(_that.field0);
    }
  }

  /// A variant of `when` that fallback to returning `null`
  ///
  /// It is equivalent to doing:
  /// ```dart
  /// switch (sealedClass) {
  ///   case Subclass(:final field):
  ///     return ...;
  ///   case _:
  ///     return null;
  /// }
  /// ```

  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? string,
    TResult? Function(PlatformInt64 field0)? int,
    TResult? Function(double field0)? double,
    TResult? Function(bool field0)? bool,
    TResult? Function()? null_,
    TResult? Function(Uint8List field0)? bytes,
  }) {
    final _that = this;
    switch (_that) {
      case YInValue_String() when string != null:
        return string(_that.field0);
      case YInValue_Int() when int != null:
        return int(_that.field0);
      case YInValue_Double() when double != null:
        return double(_that.field0);
      case YInValue_Bool() when bool != null:
        return bool(_that.field0);
      case YInValue_Null() when null_ != null:
        return null_();
      case YInValue_Bytes() when bytes != null:
        return bytes(_that.field0);
      case _:
        return null;
    }
  }
}

/// @nodoc

class YInValue_String extends YInValue {
  const YInValue_String(this.field0) : super._();

  final String field0;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YInValue_StringCopyWith<YInValue_String> get copyWith =>
      _$YInValue_StringCopyWithImpl<YInValue_String>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YInValue_String &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'YInValue.string(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YInValue_StringCopyWith<$Res>
    implements $YInValueCopyWith<$Res> {
  factory $YInValue_StringCopyWith(
          YInValue_String value, $Res Function(YInValue_String) _then) =
      _$YInValue_StringCopyWithImpl;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class _$YInValue_StringCopyWithImpl<$Res>
    implements $YInValue_StringCopyWith<$Res> {
  _$YInValue_StringCopyWithImpl(this._self, this._then);

  final YInValue_String _self;
  final $Res Function(YInValue_String) _then;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YInValue_String(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class YInValue_Int extends YInValue {
  const YInValue_Int(this.field0) : super._();

  final PlatformInt64 field0;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YInValue_IntCopyWith<YInValue_Int> get copyWith =>
      _$YInValue_IntCopyWithImpl<YInValue_Int>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YInValue_Int &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'YInValue.int(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YInValue_IntCopyWith<$Res>
    implements $YInValueCopyWith<$Res> {
  factory $YInValue_IntCopyWith(
          YInValue_Int value, $Res Function(YInValue_Int) _then) =
      _$YInValue_IntCopyWithImpl;
  @useResult
  $Res call({PlatformInt64 field0});
}

/// @nodoc
class _$YInValue_IntCopyWithImpl<$Res> implements $YInValue_IntCopyWith<$Res> {
  _$YInValue_IntCopyWithImpl(this._self, this._then);

  final YInValue_Int _self;
  final $Res Function(YInValue_Int) _then;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YInValue_Int(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as PlatformInt64,
    ));
  }
}

/// @nodoc

class YInValue_Double extends YInValue {
  const YInValue_Double(this.field0) : super._();

  final double field0;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YInValue_DoubleCopyWith<YInValue_Double> get copyWith =>
      _$YInValue_DoubleCopyWithImpl<YInValue_Double>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YInValue_Double &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'YInValue.double(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YInValue_DoubleCopyWith<$Res>
    implements $YInValueCopyWith<$Res> {
  factory $YInValue_DoubleCopyWith(
          YInValue_Double value, $Res Function(YInValue_Double) _then) =
      _$YInValue_DoubleCopyWithImpl;
  @useResult
  $Res call({double field0});
}

/// @nodoc
class _$YInValue_DoubleCopyWithImpl<$Res>
    implements $YInValue_DoubleCopyWith<$Res> {
  _$YInValue_DoubleCopyWithImpl(this._self, this._then);

  final YInValue_Double _self;
  final $Res Function(YInValue_Double) _then;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YInValue_Double(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as double,
    ));
  }
}

/// @nodoc

class YInValue_Bool extends YInValue {
  const YInValue_Bool(this.field0) : super._();

  final bool field0;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YInValue_BoolCopyWith<YInValue_Bool> get copyWith =>
      _$YInValue_BoolCopyWithImpl<YInValue_Bool>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YInValue_Bool &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @override
  String toString() {
    return 'YInValue.bool(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YInValue_BoolCopyWith<$Res>
    implements $YInValueCopyWith<$Res> {
  factory $YInValue_BoolCopyWith(
          YInValue_Bool value, $Res Function(YInValue_Bool) _then) =
      _$YInValue_BoolCopyWithImpl;
  @useResult
  $Res call({bool field0});
}

/// @nodoc
class _$YInValue_BoolCopyWithImpl<$Res>
    implements $YInValue_BoolCopyWith<$Res> {
  _$YInValue_BoolCopyWithImpl(this._self, this._then);

  final YInValue_Bool _self;
  final $Res Function(YInValue_Bool) _then;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YInValue_Bool(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as bool,
    ));
  }
}

/// @nodoc

class YInValue_Null extends YInValue {
  const YInValue_Null() : super._();

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is YInValue_Null);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  String toString() {
    return 'YInValue.null_()';
  }
}

/// @nodoc

class YInValue_Bytes extends YInValue {
  const YInValue_Bytes(this.field0) : super._();

  final Uint8List field0;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @pragma('vm:prefer-inline')
  $YInValue_BytesCopyWith<YInValue_Bytes> get copyWith =>
      _$YInValue_BytesCopyWithImpl<YInValue_Bytes>(this, _$identity);

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is YInValue_Bytes &&
            const DeepCollectionEquality().equals(other.field0, field0));
  }

  @override
  int get hashCode =>
      Object.hash(runtimeType, const DeepCollectionEquality().hash(field0));

  @override
  String toString() {
    return 'YInValue.bytes(field0: $field0)';
  }
}

/// @nodoc
abstract mixin class $YInValue_BytesCopyWith<$Res>
    implements $YInValueCopyWith<$Res> {
  factory $YInValue_BytesCopyWith(
          YInValue_Bytes value, $Res Function(YInValue_Bytes) _then) =
      _$YInValue_BytesCopyWithImpl;
  @useResult
  $Res call({Uint8List field0});
}

/// @nodoc
class _$YInValue_BytesCopyWithImpl<$Res>
    implements $YInValue_BytesCopyWith<$Res> {
  _$YInValue_BytesCopyWithImpl(this._self, this._then);

  final YInValue_Bytes _self;
  final $Res Function(YInValue_Bytes) _then;

  /// Create a copy of YInValue
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  $Res call({
    Object? field0 = null,
  }) {
    return _then(YInValue_Bytes(
      null == field0
          ? _self.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as Uint8List,
    ));
  }
}

// dart format on
